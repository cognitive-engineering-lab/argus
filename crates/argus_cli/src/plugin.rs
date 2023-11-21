use std::{borrow::Cow, env, process::{exit, Command}, time::Instant, path::PathBuf};

use clap::{Parser, Subcommand};
use rustc_errors::Handler;
use rustc_interface::interface::Result as RustcResult;
use rustc_middle::ty::{TyCtxt};
use rustc_hir::{BodyId, FnSig};
use rustc_plugin::{CrateFilter, RustcPlugin, RustcPluginArgs, Utf8Path};
use rustc_utils::{
  mir::borrowck_facts,
  errors::silent_emitter::SilentEmitter,
  source_map::{
    filename::Filename,
    find_bodies::{find_enclosing_bodies, find_bodies},
    range::{CharPos, CharRange, FunctionIdentifier, ToSpan},
  },
  timer::elapsed,
};
use log::{debug, info};
use serde::{self, Deserialize, Serialize};
use fluid_let::fluid_set;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Serialize, Deserialize)]
#[clap(version = VERSION)]
pub struct ArgusPluginArgs {
  #[clap(subcommand)]
  command: ArgusCommand,
}

#[derive(Subcommand, Serialize, Deserialize)]
enum ArgusCommand {
  Tree {
    file: String,
    line: usize,
    column: usize,
  },
  Obligations {
    file: String,
  },
  Preload,
  RustcVersion,
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

struct ArgusCallbacks<A: ArgusAnalysis, T: ToSpan, F: FnOnce() -> Option<T>> {
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
      Tree { file, line, column } => {
        todo!()
        // let compute_target = || {
        //   let cpos = CharPos {
        //     line,
        //     column,
        //   };
        //   let range = CharRange {
        //     start: cpos,
        //     end: cpos,
        //     filename: Filename::intern(&file),
        //   };

        //   FunctionIdentifier::Range(range)
        // };

        // postprocess(run(argus::analysis::tree, Some(compute_target), &compiler_args))
      }
      Obligations { .. } => {
        let nothing = || -> Option<CharRange> { None };
        postprocess(run(argus::analysis::obligations, nothing, &compiler_args))

      }
      _ => unreachable!(),
    }
  }
}

fn run<A: ArgusAnalysis, T: ToSpan>(
  analysis: A,
  compute_target: impl FnOnce() -> Option<T> + Send,
  args: &[String],
) -> ArgusResult<Vec<A::Output>> {
  let mut callbacks = ArgusCallbacks {
    analysis: Some(analysis),
    compute_target: Some(compute_target),
    output: Vec::new(),
    rustc_start: Instant::now(),
  };

  info!("Starting rustc analysis...");

  run_with_callbacks(args, &mut callbacks)?;

  Ok(callbacks.output)

    // .unwrap()
    // .map_err(|e| ArgusError::AnalysisError {
    //   error: e.to_string(),
    // })
}

pub fn run_with_callbacks(
  args: &[String],
  callbacks: &mut (dyn rustc_driver::Callbacks + Send),
) -> ArgusResult<()> {
  let mut args = args.to_vec();
  args.extend(
    // "-Z identify-regions -Z mir-opt-level=0 -Z track-diagnostics=yes -Z maximal-hir-to-mir-coverage -Z trait-solver=next -A warnings"
    //
    "-Z identify-regions -Z trait-solver=next -Z track-trait-obligations -A warnings"
      .split(' ')
      .map(|s| s.to_owned()),
  );

  log::debug!("Running command with callbacks: {args:?}");

  let compiler = rustc_driver::RunCompiler::new(&args, callbacks);

  log::debug!("building compiler ...");

  // Argus works even when the compiler exits with an error.
  let _ = compiler.run();

  Ok(())
}

fn postprocess<T: Serialize>(result: T) -> RustcResult<()> {
  println!("{}", serde_json::to_string(&result).unwrap());
  Ok(())
}

impl<A: ArgusAnalysis, T: ToSpan, F: FnOnce() -> Option<T>> rustc_driver::Callbacks for ArgusCallbacks<A, T, F> {
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

      let mut inner = |body| {
        match analysis.analyze(tcx, body) {
          Ok(v) => Some(v),
          Err(_) => None,
        }
      };

      self.output = match (self.compute_target.take().unwrap())() {
        Some(target) => {
          let target = target.to_span(tcx).expect("failed to turn target into span");
          debug!("target span: {target:?}");

          fluid_set!(argus::analysis::OBLIGATION_TARGET_SPAN, target);

          find_enclosing_bodies(tcx, target).filter_map(inner).collect::<Vec<_>>()
        },
        None => {
          debug!("no target span");

          find_bodies(tcx).into_iter().filter_map(|(_, body)| inner(body)).collect::<Vec<_>>()
        }
      }
    });

    rustc_driver::Compilation::Stop
  }
}
