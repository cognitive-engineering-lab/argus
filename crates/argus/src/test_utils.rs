use std::{env, fs, io, panic, path::Path, process::Command, sync::Arc};

use anyhow::{Context, Result};
use rustc_errors::DiagCtxt;
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
  emitter::SilentEmitter,
  proof_tree::SerializedTree,
  types::{
    intermediate::{Forgettable, FullData},
    ObligationHash, ObligationsInBody, Target,
  },
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

  let cfg = TestFileConfig::default();

  Ok((source, cfg))
}

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
          analysis::body_data(tcx, body_id).expect("failed to get obligations");

        let missing_data_obligations = obligations_in_body
          .obligations
          .iter()
          .filter(|obl| !full_data.iter().any(|(_, hash, _)| hash == obl.hash))
          .collect::<Vec<_>>();

        assert!(
          missing_data_obligations.is_empty(),
          "missing data for {:?}",
          missing_data_obligations
        );

        assert_pass(full_data, obligations_in_body);
      })
    });
    Ok(())
  };

  inner().unwrap()
}

pub fn test_locate_tree<'a, 'tcx: 'a>(
  hash: ObligationHash,
  needs_search: bool,
  thunk: impl FnOnce() -> (&'a FullData<'tcx>, &'a ObligationsInBody),
) -> Result<SerializedTree> {
  analysis::entry::pick_tree(hash, needs_search, thunk)
}

pub fn test_tree_for_target(
  path: &Path,
  mut range: CharRange,
  hash: ObligationHash,
  is_synthetic: bool,
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
        "only one body must match a body range {:?}",
        body_span
      );

      let body_id = bodies.first().unwrap();
      let target = Target {
        span: body_span,
        hash,
        is_synthetic,
      };

      let tree_opt = analysis::OBLIGATION_TARGET
        .set(target, || analysis::tree(tcx, *body_id));
      assert_pass(tree_opt);
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

      if let Err(_) = res {
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
    .for_each(|(_, body_id)| f(body_id, tcx))
}

pub fn compile_normal(
  input: impl Into<String>,
  callbacks: impl FnOnce(TyCtxt<'_>) + Send,
) {
  compile(
    input,
    &format!("--crate-type lib --sysroot {}", &*SYSROOT),
    callbacks,
  )
}

#[allow(unused_must_use)]
pub fn compile(
  input: impl Into<String>,
  args: &str,
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
      sess.dcx = DiagCtxt::with_emitter(SilentEmitter::boxed());
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
