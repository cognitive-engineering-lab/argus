use std::{
  borrow::Cow,
  env, io,
  path::PathBuf,
  process::{exit, Command},
  time::Instant,
};

use argus::{
  emitter::SilentEmitter,
  types::{ObligationHash, ToTarget},
};
use clap::{Parser, Subcommand};
use fluid_let::fluid_set;
use log::{debug, info};
use rustc_errors::DiagCtxt;
use rustc_hir::BodyId;
use rustc_interface::interface::Result as RustcResult;
use rustc_middle::ty::TyCtxt;
use rustc_plugin::{CrateFilter, RustcPlugin, RustcPluginArgs, Utf8Path};
use rustc_span::{FileName, RealFileName};
use rustc_utils::{
  source_map::{
    filename::Filename,
    find_bodies::{find_bodies, find_enclosing_bodies},
    range::{CharPos, CharRange},
  },
  timer::elapsed,
};
use serde::{self, Deserialize, Serialize};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Serialize, Deserialize)]
#[clap(version = VERSION)]
pub struct ArgusPluginArgs {
  #[clap(subcommand)]
  command: ArgusCommand,
}

#[derive(Subcommand, Serialize, Deserialize)]
enum ArgusCommand {
  Preload,
  RustcVersion,
  Obligations {
    file: Option<String>,
  },
  Tree {
    file: String,
    id: ObligationHash,
    // Represents enclosing body `CharRange`
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    is_synthetic: Option<bool>,
  },
}

trait ArgusAnalysis: Sized + Send + Sync {
  type Output: Serialize + Send + Sync;
  fn analyze(
    &mut self,
    tcx: TyCtxt,
    id: BodyId,
  ) -> anyhow::Result<Self::Output>;
}

impl<O, F> ArgusAnalysis for F
where
  for<'tcx> F: Fn(TyCtxt<'tcx>, BodyId) -> anyhow::Result<O> + Send + Sync,
  O: Serialize + Send + Sync,
{
  type Output = O;
  fn analyze<'tcx>(
    &mut self,
    tcx: TyCtxt<'tcx>,
    id: BodyId,
  ) -> anyhow::Result<Self::Output> {
    (self)(tcx, id)
  }
}

struct ArgusCallbacks<A: ArgusAnalysis, T: ToTarget, F: FnOnce() -> Option<T>> {
  file: Option<PathBuf>,
  analysis: Option<A>,
  compute_target: Option<F>,
  result: Vec<A::Output>,
  rustc_start: Instant,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum ArgusError {
  BuildError { range: Option<CharRange> },
  AnalysisError { error: String },
}

pub type ArgusResult<T> = std::result::Result<T, ArgusError>;

pub struct ArgusPlugin;
impl RustcPlugin for ArgusPlugin {
  type Args = ArgusPluginArgs;

  fn version(&self) -> Cow<'static, str> {
    env!("CARGO_PKG_VERSION").into()
  }

  fn driver_name(&self) -> Cow<'static, str> {
    "argus-driver".into()
  }

  fn args(&self, target_dir: &Utf8Path) -> RustcPluginArgs<ArgusPluginArgs> {
    let args = ArgusPluginArgs::parse_from(env::args().skip(1));

    let cargo_path =
      env::var("CARGO_PATH").unwrap_or_else(|_| "cargo".to_string());

    use ArgusCommand::*;
    match &args.command {
      Preload => {
        let mut cmd = Command::new(cargo_path);
        // Note: this command must share certain parameters with rustc_plugin so Cargo will not recompute
        // dependencies when actually running the driver, e.g. RUSTFLAGS.
        cmd
          .args(["check", "--all", "--all-features", "--target-dir"])
          .arg(target_dir);
        let exit_status = cmd.status().expect("could not run cargo");
        exit(exit_status.code().unwrap_or(-1));
      }
      RustcVersion => {
        let commit_hash =
          rustc_interface::util::rustc_version_str().unwrap_or("unknown");
        println!("{commit_hash}");
        exit(0);
      }
      _ => {}
    };

    let file = match &args.command {
      Tree { file, .. } => Some(file),
      Obligations { file } => file.as_ref(),
      _ => unreachable!(),
    };

    let filter = file
      .map(|file| CrateFilter::CrateContainingFile(PathBuf::from(file)))
      .unwrap_or(CrateFilter::OnlyWorkspace);

    RustcPluginArgs { filter, args }
  }

  fn run(
    self,
    compiler_args: Vec<String>,
    plugin_args: ArgusPluginArgs,
  ) -> RustcResult<()> {
    use ArgusCommand::*;
    match plugin_args.command {
      Tree {
        file,
        id,
        start_line,
        start_column,
        end_line,
        end_column,

        // TODO: we dono't yet handle synthetic queries in Argus.
        is_synthetic,
      } => {
        let is_synthetic = is_synthetic.unwrap_or(false);
        let compute_target = || {
          Some((
            id,
            CharRange {
              start: CharPos {
                line: start_line,
                column: start_column,
              },
              end: CharPos {
                line: end_line,
                column: end_column,
              },
              filename: Filename::intern(&file),
            },
            is_synthetic,
          ))
        };

        let v = run(
          argus::analysis::tree,
          Some(PathBuf::from(&file)),
          compute_target,
          &compiler_args,
        );
        postprocess(v)
      }
      Obligations { file, .. } => {
        let nothing = || None::<(ObligationHash, CharRange)>;
        let v = run(
          argus::analysis::obligations,
          file.map(PathBuf::from),
          nothing,
          &compiler_args,
        );
        postprocess(v)
      }
      _ => unreachable!(),
    }
  }
}

fn run<A: ArgusAnalysis, T: ToTarget>(
  analysis: A,
  file: Option<PathBuf>,
  compute_target: impl FnOnce() -> Option<T> + Send,
  args: &[String],
) -> ArgusResult<Vec<A::Output>> {
  let mut callbacks = ArgusCallbacks {
    file,
    analysis: Some(analysis),
    compute_target: Some(compute_target),
    result: Vec::default(),
    rustc_start: Instant::now(),
  };

  info!("Starting rustc analysis...");

  #[allow(unused_must_use)]
  let _ = run_with_callbacks(args, &mut callbacks);

  Ok(callbacks.result)
}

pub fn run_with_callbacks(
  args: &[String],
  callbacks: &mut (dyn rustc_driver::Callbacks + Send),
) -> ArgusResult<()> {
  let mut args = args.to_vec();
  args.extend(
    "-Z next-solver -Z print-type-sizes=true -A warnings"
      .split(' ')
      .map(|s| s.to_owned()),
  );

  log::debug!("Running command with callbacks: {args:?}");

  let compiler = rustc_driver::RunCompiler::new(&args, callbacks);

  log::debug!("Building compiler ...");

  // Argus works even when the compiler exits with an error.
  #[allow(unused_must_use)]
  let _ = compiler
    .run()
    .map_err(|_| ArgusError::BuildError { range: None });

  Ok(())
}

fn postprocess<T: Serialize>(result: T) -> RustcResult<()> {
  serde_json::to_writer(io::stdout(), &result).unwrap();
  Ok(())
}

impl<A: ArgusAnalysis, T: ToTarget, F: FnOnce() -> Option<T>>
  rustc_driver::Callbacks for ArgusCallbacks<A, T, F>
{
  fn config(&mut self, config: &mut rustc_interface::Config) {
    config.parse_sess_created = Some(Box::new(|sess| {
      // // Create a new emitter writer which consumes *silently* all
      // // errors. There most certainly is a *better* way to do this,
      // // if you, the reader, know what that is, please open an issue :)
      // let fallback_bundle = rustc_errors::fallback_fluent_bundle(
      //   rustc_driver::DEFAULT_LOCALE_RESOURCES.to_vec(),
      //   false,
      // );
      // let emitter = HumanEmitter::new(Box::new(io::sink()), fallback_bundle);
      // sess.dcx = DiagCtxt::with_emitter(Box::new(emitter));

      sess.dcx = DiagCtxt::with_emitter(SilentEmitter::boxed());
    }));
  }

  fn after_expansion<'tcx>(
    &mut self,
    _compiler: &rustc_interface::interface::Compiler,
    queries: &'tcx rustc_interface::Queries<'tcx>,
  ) -> rustc_driver::Compilation {
    elapsed("rustc", self.rustc_start);
    let start = Instant::now();

    queries.global_ctxt().unwrap().enter(|tcx| {
      elapsed("global_ctxt", start);
      let mut analysis = self.analysis.take().unwrap();
      let target_file = self.file.as_ref();

      let mut inner = |(_, body)| {
        if let FileName::Real(RealFileName::LocalPath(p)) =
          get_file_of_body(tcx, body)
        {
          if target_file.map(|f| f.ends_with(&p)).unwrap_or(true) {
            debug!("analyzing {:?}", body);
            match analysis.analyze(tcx, body) {
              Ok(v) => Some(v),
              Err(_) => None,
            }
          } else {
            debug!("Skipping file {:?} due to target {:?}", p, self.file);
            None
          }
        } else {
          None
        }
      };

      self.result = match (self.compute_target.take().unwrap())() {
        Some(target) => {
          log::debug!("Getting target");
          let target = target.to_target(tcx).expect("Couldn't compute target");
          log::debug!("Got target");
          let body_span = target.span.clone();

          debug!("target: {target:?}");

          fluid_set!(argus::analysis::OBLIGATION_TARGET, target);

          find_enclosing_bodies(tcx, body_span)
            .filter_map(|b| inner((body_span, b)))
            .collect::<Vec<_>>()
        }
        None => {
          debug!("no target");
          find_bodies(tcx)
            .into_iter()
            .filter_map(inner)
            .collect::<Vec<_>>()
        }
      };
    });

    rustc_driver::Compilation::Stop
  }
}

fn get_file_of_body(tcx: TyCtxt<'_>, body_id: rustc_hir::BodyId) -> FileName {
  let hir = tcx.hir();
  let body_span = hir.body(body_id).value.span;
  let source_map = tcx.sess.source_map();
  source_map.span_to_filename(body_span)
}
