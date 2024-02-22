use std::{env, fmt::Debug, fs, path::Path, process::Command, sync::Once};

use anyhow::{ensure, Context, Result};

static SETUP: Once = Once::new();

fn run<P: AsRef<Path>>(dir: P, f: impl FnOnce(&mut Command)) -> Result<String> {
  let root = env::temp_dir().join("argus");
  let heredir = Path::new(".").canonicalize()?;

  SETUP.call_once(|| {
    let mut cmd = Command::new("cargo");
    cmd.args(["install", "--path", ".", "--debug", "--locked", "--root"]);
    cmd.arg(&root);
    cmd.current_dir(&heredir);
    let status = cmd.status().unwrap();
    if !status.success() {
      panic!("installing argus failed")
    }
  });

  let mut cmd = Command::new("cargo");
  cmd.arg("argus");
  cmd.arg("obligations");
  // Don't specify a file to analyze all local crates.

  let path = format!(
    "{}:{}",
    root.join("bin").display(),
    env::var("PATH").unwrap_or_else(|_| "".into())
  );
  cmd.env("PATH", path);

  let ws = heredir.join("tests").join(dir);
  cmd.current_dir(&ws);

  f(&mut cmd);

  let _ = fs::remove_dir_all(ws.join("target"));

  let output = cmd.output().context("Process failed")?;
  ensure!(
    output.status.success(),
    "Process exited with non-zero exit code. Stderr:\n{}",
    String::from_utf8(output.stderr)?
  );

  Ok(String::from_utf8(output.stdout)?)
}

macro_rules! mk_tests_for {
    ($($i:ident),*) => {$(
        #[test]
        fn $i() -> Result<()> {
            _ = run(format!("workspaces/{}", stringify!($i)), |_cmd| {})?;
            Ok(())
        }
    )*}
}

mk_tests_for! {
  axum,
  bevy,
  // chumsky, // NOTE: as of now this consumes too much memory
  diesel,
  easy_ml,
  entrait,
  nalgebra,
  uom
}

// TODO: include individual test if we want to see a particular output
//
// We should also specify some type of "blessed output,"
// to make sure that certain errors are present at the right locations.
