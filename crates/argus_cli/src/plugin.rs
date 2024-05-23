use std::{
  borrow::Cow,
  env, io,
  path::PathBuf,
  process::{exit, Command},
  time::Instant,
};

use argus_lib::{
  analysis,
  ext::TyCtxtExt as ArgusTyCtxtExt,
  find_bodies::{find_bodies, find_enclosing_bodies},
  types::{ObligationHash, ToTarget},
};
use clap::{Parser, Subcommand};
use fluid_let::fluid_set;
use rustc_hir::BodyId;
use rustc_interface::interface::Result as RustcResult;
use rustc_middle::ty::TyCtxt;
use rustc_plugin::{CrateFilter, RustcPlugin, RustcPluginArgs, Utf8Path};
use rustc_span::{FileName, RealFileName};
use rustc_utils::{
  source_map::{
    filename::Filename,
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

  #[clap(long)]
  show_stderr: bool,
}

#[derive(Subcommand, Serialize, Deserialize)]
enum ArgusCommand {
  Preload,
  RustcVersion,
  Bundle,
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
  show_stderr: bool,
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
      Bundle => None,
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
    match &plugin_args.command {
      Tree {
        file,
        id,
        start_line,
        start_column,
        end_line,
        end_column,
        is_synthetic,
      } => {
        let is_synthetic = is_synthetic.unwrap_or(false);
        let compute_target = || {
          Some((
            id,
            CharRange {
              start: CharPos {
                line: *start_line,
                column: *start_column,
              },
              end: CharPos {
                line: *end_line,
                column: *end_column,
              },
              filename: Filename::intern(&file),
            },
            is_synthetic,
          ))
        };

        let v = run(
          analysis::tree,
          Some(PathBuf::from(&file)),
          compute_target,
          &plugin_args,
          &compiler_args,
        );
        postprocess(v)
      }
      Obligations { file, .. } => {
        let nothing = || None::<(ObligationHash, CharRange)>;
        let v = run(
          analysis::obligations,
          file.as_ref().map(PathBuf::from),
          nothing,
          &plugin_args,
          &compiler_args,
        );
        postprocess(v)
      }
      Bundle => {
        let nothing = || None::<(ObligationHash, CharRange)>;
        let v = run(
          analysis::bundle,
          None,
          nothing,
          &plugin_args,
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
  plugin_args: &ArgusPluginArgs,
  args: &[String],
) -> ArgusResult<Vec<A::Output>> {
  let mut callbacks = ArgusCallbacks {
    file,
    show_stderr: plugin_args.show_stderr,
    analysis: Some(analysis),
    compute_target: Some(compute_target),
    result: Vec::default(),
    rustc_start: Instant::now(),
  };

  log::info!("Starting rustc analysis...");

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
    if self.show_stderr {
      return;
    }

    config.psess_created = Some(Box::new(|sess| {
      let fallback_bundle = rustc_errors::fallback_fluent_bundle(
        rustc_driver::DEFAULT_LOCALE_RESOURCES.to_vec(),
        false,
      );

      sess.dcx.make_silent(fallback_bundle, None, false);
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
          tcx.body_filename(body)
        {
          if target_file.map(|f| f.ends_with(&p)).unwrap_or(true) {
            log::info!("analyzing {:?}", body);
            match analysis.analyze(tcx, body) {
              Ok(v) => Some(v),
              Err(e) => {
                log::error!("Error analyzing body {:?} {:?}", body, e);
                None
              }
            }
          } else {
            log::debug!("Skipping file {:?} due to target {:?}", p, self.file);
            None
          }
        } else {
          None
        }
      };

      self.result = match (self.compute_target.take().unwrap())() {
        Some(target) => {
          let target = target.to_target(tcx).expect("Couldn't compute target");
          let body_span = target.span.clone();
          fluid_set!(analysis::OBLIGATION_TARGET, target);

          find_enclosing_bodies(tcx, body_span)
            .filter_map(|b| inner((body_span, b)))
            .collect::<Vec<_>>()
        }
        None => find_bodies(tcx)
          .into_iter()
          .filter_map(inner)
          .collect::<Vec<_>>(),
      };
    });

    rustc_driver::Compilation::Stop
  }
}
