use std::{
  collections::HashMap, env, fs, io, panic, path::Path, process::Command,
  sync::Arc,
};

use anyhow::{bail, Context, Result};
use fluid_let::fluid_set;
use itertools::Itertools;
use rustc_errors::DiagCtxt;
use rustc_hir::BodyId;
use rustc_middle::{
  mir::{Rvalue, StatementKind},
  ty::TyCtxt,
};
use rustc_span::source_map::FileLoader;
use rustc_utils::{
  errors::silent_emitter::SilentEmitter,
  source_map::{
    filename::{Filename, FilenameIndex},
    find_bodies::{find_bodies, find_enclosing_bodies},
    range::{self, CharPos, CharRange, ToSpan},
    spanner::Spanner,
  },
  timer::elapsed,
  BodyExt, OperandExt,
};

lazy_static::lazy_static! {
  static ref SYSROOT: String = {
    let rustc_output = Command::new("rustc")
      .args(["--print", "sysroot"])
      .output()
      .unwrap()
      .stdout;
    String::from_utf8(rustc_output).unwrap().trim().to_owned()
  };
}

static CFG_HASH: &str = "////!";

pub const DUMMY_FILE_NAME: &str = "dummy.rs";

thread_local! {
  pub static DUMMY_FILE: FilenameIndex = Filename::intern(DUMMY_FILE_NAME);
}

struct StringLoader(String);
impl FileLoader for StringLoader {
  fn file_exists(&self, _: &Path) -> bool {
    true
  }

  fn read_file(&self, _: &Path) -> io::Result<String> {
    Ok(self.0.clone())
  }

  fn read_binary_file(&self, path: &Path) -> io::Result<Arc<[u8]>> {
    // FIXME: there must be a better way to do this.
    let vec_data = fs::read(path)?;
    let boxed_data: Box<[u8]> = vec_data.into_boxed_slice();
    Ok(Arc::from(boxed_data))
  }
}

#[derive(Debug, Default)]
pub(crate) struct TestFileConfig {}

pub(crate) fn load_test_from_file(
  path: &Path,
) -> Result<(String, TestFileConfig)> {
  log::info!(
    "Loading test from {}",
    path.file_name().unwrap().to_string_lossy()
  );
  let c = fs::read(path)
    .with_context(|| format!("failed to load test from {path:?}"))?;
  let source = String::from_utf8(c)
    .with_context(|| format!("UTF8 parse error in file: {path:?}"))?;

  let mut cfg = TestFileConfig::default();

  Ok((source, cfg))
}

pub fn test_obligations_no_crash(
  path: &Path,
  assert_pass: impl Fn() + Send + Sync + Copy,
) {
  let inner = || -> Result<()> {
    let (source, cfg) = load_test_from_file(path)?;
    compile_normal(source, move |tcx| {
      for_each_body(tcx, |body_id, _body_with_facts| {
        // TODO: actually generate the obligations in the file.
        assert_pass();
      })
    });
    Ok(())
  };

  inner().unwrap()
}

pub fn run_in_dir(
  dir: impl AsRef<Path>,
  test_fn: impl Fn(&Path) + std::panic::RefUnwindSafe,
) {
  let main = || -> Result<()> {
    let test_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("tests")
      .join(dir.as_ref());
    let only = env::var("ONLY").ok();
    let tests = fs::read_dir(test_dir)?;
    let mut failed = false;
    let mut passed = 0;
    let mut total = 0;
    for test in tests {
      let path = test?.path();
      let test_name = path.file_name().unwrap().to_str().unwrap();

      if let Some(only) = &only {
        if !test_name.starts_with(only) {
          continue;
        }
      }

      let res = panic::catch_unwind(|| test_fn(&path));

      if let Err(e) = res {
        failed = true;
        eprintln!(
          r#"

          !! \x1b[31m{test_name}\x1b[0m\n\t{e:?}

          "#
        );
      } else {
        passed += 1;
      }
      total += 1;
    }

    log::info!(
      r#"
      · {} / {} succeeded in {:?}

      "#,
      passed,
      total,
      dir.as_ref(),
    );

    assert!(!failed, "some tests failed");

    Ok(())
  };

  main().unwrap();
}

pub fn for_each_body(tcx: TyCtxt, mut f: impl FnMut(BodyId, TyCtxt)) {
  find_bodies(tcx)
    .into_iter()
    .for_each(|(_, body_id)| f(body_id, tcx))
}

pub fn compile_normal(
  input: impl Into<String>,
  callbacks: impl FnOnce(TyCtxt<'_>) + Send,
) {
  compile(
    input,
    &format!("--crate-type lib --sysroot {}", &*SYSROOT),
    false,
    callbacks,
  )
}

#[allow(unused_must_use)]
pub fn compile(
  input: impl Into<String>,
  args: &str,
  is_interpreter: bool,
  callback: impl FnOnce(TyCtxt<'_>) + Send,
) {
  let mut callbacks = TestCallbacks {
    callback: Some(callback),
  };
  let args = format!(
    "rustc {DUMMY_FILE_NAME} --edition=2021 -Z next-solver -A warnings {args}",
  );
  let args = args.split(' ').map(|s| s.to_string()).collect::<Vec<_>>();

  // Explicitly ignore the unused return value. Many test cases are intended
  // to fail compilation, but the analysis results should still be sound.
  rustc_driver::catch_fatal_errors(|| {
    let mut compiler = rustc_driver::RunCompiler::new(&args, &mut callbacks);
    compiler.set_file_loader(Some(Box::new(StringLoader(input.into()))));
    compiler.run()
  });
}

struct TestCallbacks<Cb> {
  callback: Option<Cb>,
}

impl<Cb> rustc_driver::Callbacks for TestCallbacks<Cb>
where
  Cb: FnOnce(TyCtxt<'_>),
{
  fn config(&mut self, config: &mut rustc_interface::Config) {
    config.parse_sess_created = Some(Box::new(|sess| {
      sess.dcx = DiagCtxt::with_emitter(Box::new(SilentEmitter));
    }));
  }

  fn after_expansion<'tcx>(
    &mut self,
    _compiler: &rustc_interface::interface::Compiler,
    queries: &'tcx rustc_interface::Queries<'tcx>,
  ) -> rustc_driver::Compilation {
    queries.global_ctxt().unwrap().enter(|tcx| {
      let callback = self.callback.take().unwrap();
      callback(tcx);
    });
    rustc_driver::Compilation::Stop
  }
}
