use std::{
  env, fs, io, panic,
  path::Path,
  process::Command,
  sync::{Arc, LazyLock},
};

use anyhow::{Context, Result};
use rustc_hir::BodyId;
use rustc_middle::ty::TyCtxt;
use rustc_span::source_map::FileLoader;
use rustc_utils::source_map::{
  filename::{Filename, FilenameIndex},
  find_bodies::{find_bodies, find_enclosing_bodies},
  range::{CharRange, ToSpan},
};

use crate::{
  analysis,
  proof_tree::SerializedTree,
  types::{
    intermediate::{Forgettable, FullData},
    ObligationHash, ObligationsInBody, Target,
  },
};

static SYSROOT: LazyLock<String> = LazyLock::new(|| {
  let rustc_output = Command::new("rustc")
    .args(["--print", "sysroot"])
    .output()
    .unwrap()
    .stdout;
  String::from_utf8(rustc_output).unwrap().trim().to_owned()
});

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
    .with_context(|| format!("failed to load test from {}", path.display()))?;
  let source = String::from_utf8(c)
    .with_context(|| format!("UTF8 parse error in file: {}", path.display()))?;

  let cfg = TestFileConfig::default();

  Ok((source, cfg))
}

#[allow(clippy::missing_panics_doc)]
pub fn test_obligations_no_crash(
  path: &Path,
  mut assert_pass: impl for<'tcx> FnMut(Forgettable<FullData<'tcx>>, ObligationsInBody)
    + Send
    + Sync,
) {
  let inner = || -> Result<()> {
    let (source, _cfg) = load_test_from_file(path)?;
    compile_normal(source, move |tcx| {
      for_each_body(tcx, |body_id, tcx| {
        let (full_data, obligations_in_body) =
          analysis::body_data(tcx, body_id);

        assert_pass(full_data, obligations_in_body);
      });
    });
    Ok(())
  };

  inner().unwrap();
}

pub fn test_locate_tree<'a, 'tcx: 'a>(
  hash: ObligationHash,
  thunk: impl FnOnce() -> (&'a FullData<'tcx>, &'a ObligationsInBody),
) -> Result<SerializedTree> {
  analysis::entry::pick_tree(hash, thunk)
}

#[allow(clippy::missing_panics_doc)]
pub fn test_tree_for_target(
  path: &Path,
  mut range: CharRange,
  hash: ObligationHash,
  mut assert_pass: impl FnMut(Result<SerializedTree>) + Send + Sync,
) {
  let inner = || -> Result<()> {
    let (source, _cfg) = load_test_from_file(path)?;
    compile_normal(source, move |tcx| {
      range.filename = DUMMY_FILE.with(|fidx| *fidx);
      let body_span = range
        .to_span(tcx)
        .expect("couldn't find span for body range");

      let bodies = find_enclosing_bodies(tcx, body_span).collect::<Vec<_>>();
      assert!(
        bodies.len() == 1,
        "only one body must match a body range {body_span:?}",
      );

      let body_id = bodies.first().unwrap();
      let target = Target {
        span: body_span,
        hash,
      };

      let tree_opt = analysis::OBLIGATION_TARGET
        .set(target, || analysis::tree(tcx, *body_id));
      assert_pass(tree_opt);
    });
    Ok(())
  };

  inner().unwrap();
}

#[allow(clippy::missing_panics_doc)]
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

      if res.is_err() {
        failed = true;
        eprintln!("\n\n\x1b[31m!! {test_name}\x1b[0m\n\n");
      } else {
        passed += 1;
      }
      total += 1;
    }

    log::info!(
      "\n\n{} / {} succeeded in {:?}\n\n",
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
    .for_each(|(_, body_id)| f(body_id, tcx));
}

pub fn compile_normal(
  input: impl Into<String>,
  callbacks: impl FnOnce(TyCtxt<'_>) + Send,
) {
  compile(
    input,
    &format!("--crate-type lib --sysroot {}", &*SYSROOT),
    callbacks,
  );
}

#[allow(unused_must_use)]
pub fn compile(
  input: impl Into<String>,
  args: &str,
  callback: impl FnOnce(TyCtxt<'_>) + Send,
) {
  let mut callbacks = TestCallbacks {
    callback: Some(callback),
    input: input.into(),
  };
  let args = format!(
    "rustc {DUMMY_FILE_NAME} --edition=2021 -Z next-solver -A warnings {args}",
  );
  let args = args.split(' ').map(ToString::to_string).collect::<Vec<_>>();

  rustc_driver::catch_fatal_errors(|| {
    rustc_driver::run_compiler(&args, &mut callbacks);
  });
}

struct TestCallbacks<Cb> {
  callback: Option<Cb>,
  input: String,
}

impl<Cb> rustc_driver::Callbacks for TestCallbacks<Cb>
where
  Cb: FnOnce(TyCtxt<'_>),
{
  fn config(&mut self, config: &mut rustc_interface::Config) {
    config.file_loader =
      Some(Box::new(StringLoader(std::mem::take(&mut self.input))));
    config.psess_created = Some(Box::new(|sess| {
      sess.dcx().make_silent(None, false);
    }));
  }

  fn after_expansion(
    &mut self,
    _compiler: &rustc_interface::interface::Compiler,
    tcx: TyCtxt,
  ) -> rustc_driver::Compilation {
    let callback = self.callback.take().unwrap();
    callback(tcx);
    rustc_driver::Compilation::Stop
  }
}
