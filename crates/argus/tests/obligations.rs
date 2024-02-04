use argus::test_utils as tu;

#[test_log::test]
fn obligations() {
  tu::run_in_dir("traits", |path| {
    let filename = path.file_name().unwrap().to_string_lossy();
    tu::test_obligations_no_crash(path, || assert!(true, "TODO"));
  });
}
