use std::{borrow::Cow, env, process::exit, time::Instant};

use clap::{Parser, Subcommand};
use rustc_interface::interface::Result as RustcResult;
use rustc_plugin::{CrateFilter, RustcPlugin, RustcPluginArgs, Utf8Path};
use rustc_utils::source_map::{find_bodies::find_bodies, range::CharRange};
use serde::{self, Deserialize, Serialize};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Parser, Serialize, Deserialize)]
#[clap(version = VERSION)]
pub struct ArgusPluginArgs {
  #[clap(subcommand)]
  command: ArgusCommand,
}

#[derive(Debug, Subcommand, Serialize, Deserialize)]
enum ArgusCommand {
  Trees,
  RustcVersion,
}

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

    log::debug!("Provided PluginArgs {args:?}");

    let cargo_path =
      env::var("CARGO_PATH").unwrap_or_else(|_| "cargo".to_string());

    use ArgusCommand::*;
    match &args.command {
      RustcVersion => {
        let commit_hash =
          rustc_interface::util::rustc_version_str().unwrap_or("unknown");
        println!("{commit_hash}");
        exit(0);
      }
      _ => {}
    };

    RustcPluginArgs {
      filter: CrateFilter::OnlyWorkspace,
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
      Trees => {
        let mut callbacks = ArgusCallbacks {
          rustc_start: Instant::now(),
        };

        log::info!("Starting rustc analysis...");
        let _ = run_with_callbacks(&compiler_args, &mut callbacks);

        Ok(())
      }
      _ => unreachable!(),
    }
  }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum ArgusError {
  BuildError { range: Option<CharRange> },
}
pub type ArgusResult<T> = ::std::result::Result<T, ArgusError>;

pub fn run_with_callbacks(
  args: &[String],
  callbacks: &mut (dyn rustc_driver::Callbacks + Send),
) -> ArgusResult<()> {
  let mut args = args.to_vec();
  args.extend(
    // "-Z identify-regions -Z mir-opt-level=0 -Z track-diagnostics=yes -Z maximal-hir-to-mir-coverage -Z trait-solver=next -A warnings"
    "-Z identify-regions -Z trait-solver=next -Z dump-solver-proof-tree=always -A warnings"
      .split(' ')
      .map(|s| s.to_owned()),
  );

  log::debug!("Running command with callbacks: {args:?}");

  let compiler = rustc_driver::RunCompiler::new(&args, callbacks);

  log::debug!("building compiler ...");

  compiler
    .run()
    .map_err(|_| ArgusError::BuildError { range: None })
}

struct ArgusCallbacks {
  rustc_start: Instant,
}

impl rustc_driver::Callbacks for ArgusCallbacks {
  fn after_expansion<'tcx>(
    &mut self,
    _compiler: &rustc_interface::interface::Compiler,
    queries: &'tcx rustc_interface::Queries<'tcx>,
  ) -> rustc_driver::Compilation {
    queries.global_ctxt().unwrap().enter(|tcx| {
      find_bodies(tcx).into_iter().for_each(|(_, body_id)| {
        argus::trees_in_body(&tcx, body_id);
      });
    });

    log::debug!("Callback analysis took {:?}", self.rustc_start.elapsed());

    rustc_driver::Compilation::Stop
  }
}
