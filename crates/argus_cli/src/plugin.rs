use std::{borrow::Cow, env, process::{exit, Command}, time::Instant, path::PathBuf};

use clap::{Parser, Subcommand};
use rustc_errors::Handler;
use rustc_interface::interface::Result as RustcResult;
use rustc_middle::ty::{TyCtxt};
use rustc_hir::{BodyId, FnSig};
use rustc_span::{FileName, RealFileName};
use rustc_plugin::{CrateFilter, RustcPlugin, RustcPluginArgs, Utf8Path};
use rustc_utils::{
  mir::borrowck_facts,
  errors::silent_emitter::SilentEmitter,
  source_map::{
    filename::Filename,
    find_bodies::{find_enclosing_bodies, find_bodies},
    range::{CharPos, CharRange, FunctionIdentifier},
  },
  timer::elapsed,
};
use log::{debug, info};
use serde::{self, Deserialize, Serialize};
use fluid_let::fluid_set;
use argus::{
  proof_tree::Obligation,
  Target, ToTarget,
};

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
    file: String,
  },
  Tree {
    file: String,
    id: String,
  },
}

trait ArgusAnalysis: Sized + Send + Sync {
  type Output: Serialize + Send + Sync;
  fn analyze(&mut self, tcx: TyCtxt, id: BodyId) -> anyhow::Result<Self::Output>;
}

impl<F, O> ArgusAnalysis for F
where
  F: for<'tcx> Fn(TyCtxt<'tcx>, BodyId) -> anyhow::Result<O> + Send + Sync,
  O: Serialize + Send + Sync,
{
  type Output = O;
  fn analyze(&mut self, tcx: TyCtxt, id: BodyId) -> anyhow::Result<Self::Output> {
    (self)(tcx, id)
  }
}

struct ArgusCallbacks<A: ArgusAnalysis, T: ToTarget, F: FnOnce() -> Option<T>> {
  file: PathBuf,
  analysis: Option<A>,
  compute_target: Option<F>,
  // FIXME: we're throwing away any failures.
  output: Vec<A::Output>,
  rustc_start: Instant,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum ArgusError {
  BuildError { range: Option<CharRange> },
  AnalysisError { error: String,  }
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
      Tree { file, .. } => file,
      Obligations { file } => file,
      _ => unreachable!(),
    };

    RustcPluginArgs {
      filter: CrateFilter::CrateContainingFile(PathBuf::from(file)),
      args,
    }
  }


  fn run(
    self,
    compiler_args: Vec<String>,
    plugin_args: ArgusPluginArgs,
  ) -> RustcResult<()> {
    use ArgusCommand::*;
    match plugin_args.command {
      Tree { file, id } => {
        let compute_target = move || {
          Some(id)
        };

        postprocess(run(PathBuf::from(file), argus::analysis::tree, compute_target, &compiler_args))
      }
      Obligations { file, .. } => {
        let nothing = || { None::<String> };
        postprocess(run(PathBuf::from(file), argus::analysis::obligations, nothing, &compiler_args))

      }
      _ => unreachable!(),
    }
  }
}

fn run<A: ArgusAnalysis, T: ToTarget>(
  file: PathBuf,
  analysis: A,
  compute_target: impl FnOnce() -> Option<T> + Send,
  args: &[String],
) -> ArgusResult<Vec<A::Output>> {
  let mut callbacks = ArgusCallbacks {
    file,
    analysis: Some(analysis),
    compute_target: Some(compute_target),
    output: Vec::new(),
    rustc_start: Instant::now(),
  };

  info!("Starting rustc analysis...");

  run_with_callbacks(args, &mut callbacks)?;

  Ok(callbacks.output)
}

pub fn run_with_callbacks(
  args: &[String],
  callbacks: &mut (dyn rustc_driver::Callbacks + Send),
) -> ArgusResult<()> {
  let mut args = args.to_vec();
  // -Z identify-regions -Z track-diagnostics=yes
  args.extend(
    "-Z trait-solver=next -Z track-trait-obligations -A warnings"
      .split(' ')
      .map(|s| s.to_owned()),
  );

  log::debug!("Running command with callbacks: {args:?}");

  let compiler = rustc_driver::RunCompiler::new(&args, callbacks);

  log::debug!("Building compiler ...");

  // Argus works even when the compiler exits with an error.
  let _ = compiler.run();

  Ok(())
}

fn postprocess<T: Serialize>(result: T) -> RustcResult<()> {
  println!("{}", serde_json::to_string(&result).unwrap());
  Ok(())
}

impl<A: ArgusAnalysis, T: ToTarget, F: FnOnce() -> Option<T>> rustc_driver::Callbacks for ArgusCallbacks<A, T, F> {
 fn config(&mut self, config: &mut rustc_interface::Config) {
    config.parse_sess_created = Some(Box::new(|sess| {
      // Create a new emitter writer which consumes *silently* all
      // errors. There most certainly is a *better* way to do this,
      // if you, the reader, know what that is, please open an issue :)
      let handler = Handler::with_emitter(Box::new(SilentEmitter));
      sess.span_diagnostic = handler;
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

      let mut inner = |(_, body)| {
        if let FileName::Real(RealFileName::LocalPath(p)) = get_file_of_body(tcx, body) {
          if p == self.file {
            match analysis.analyze(tcx, body) {
              Ok(v) => Some(v),
              Err(_) => None,
            }
          } else {
            log::info!("Skipping file {:?} as it isn't {:?}", p, self.file);
            None
          }
        } else {
          None
        }
      };

      self.output = match (self.compute_target.take().unwrap())() {
        Some(target) => {
          let target = target.to_target();

          debug!("target: {target:?}");

          fluid_set!(argus::analysis::OBLIGATION_TARGET, target);

          find_bodies(tcx).into_iter().filter_map(inner).collect::<Vec<_>>()
        },
        None => {
          debug!("no target");

          find_bodies(tcx).into_iter().filter_map(inner).collect::<Vec<_>>()
        }
      }
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
