use std::{
    borrow::Cow,
    env,
    process::{exit, Command},
    time::Instant,
};

use clap::{Parser, Subcommand};
use fluid_let::fluid_set;
use rustc_hir::BodyId;
use rustc_interface::interface::Result as RustcResult;
use rustc_middle::ty::TyCtxt;
use rustc_plugin::{CrateFilter, RustcPlugin, RustcPluginArgs, Utf8Path};
use rustc_utils::{mir::borrowck_facts, source_map::find_bodies::find_bodies};
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

        let cargo_path = env::var("CARGO_PATH").unwrap_or_else(|_| "cargo".to_string());

        use ArgusCommand::*;
        match &args.command {
            RustcVersion => {
                let commit_hash = rustc_interface::util::rustc_version_str().unwrap_or("unknown");
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

    fn run(self, compiler_args: Vec<String>, plugin_args: ArgusPluginArgs) -> RustcResult<()> {
        use ArgusCommand::*;
        match plugin_args.command {
            _ => unreachable!(),
        }
    }
}

// fn permissions_analyze_body(
//   tcx: TyCtxt,
//   id: BodyId,
// ) -> AquascopeResult<analysis::AnalysisOutput> {
//   analysis::AquascopeAnalysis::run(tcx, id)
// }

// fn postprocess<T: Serialize>(result: T) -> RustcResult<()> {
//   println!("{}", serde_json::to_string(&result).unwrap());
//   Ok(())
// }

#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ArgusError {}
pub type ArgusResult<T> = ::std::result::Result<T, ArgusError>;

pub fn run_with_callbacks(
    args: &[String],
    callbacks: &mut (dyn rustc_driver::Callbacks + Send),
) -> AquascopeResult<()> {
    let mut args = args.to_vec();
    args.extend(
        // "-Z identify-regions -Z mir-opt-level=0 -Z track-diagnostics=yes -Z maximal-hir-to-mir-coverage -Z trait-solver=next -A warnings"
        "-Z identify-regions -Z trait-solver=next -A warnings"
            .split(' ')
            .map(|s| s.to_owned()),
    );

    log::debug!("Running command with callbacks: {args:?}");

    let compiler = rustc_driver::RunCompiler::new(&args, callbacks);

    log::debug!("building compiler ...");

    compiler
        .run()
        .map_err(|_| AquascopeError::BuildError { range: None })
}
