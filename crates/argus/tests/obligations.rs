use argus_lib::test_utils as tu;

#[test_log::test]
fn obligations() {
  tu::run_in_dir("contained", |path| {
    tu::test_obligations_no_crash(path, |full_data, obligations| {
      let mut missing = vec![];
      let t = (&*full_data, &obligations);

      for obl in t.1.obligations.iter() {
        let res = tu::test_locate_tree(obl.hash, true, || t);
        if res.is_err() {
          missing.push((res, obl))
        }
      }

      assert!(
        missing.is_empty(),
        "\n\nmissing {} / {} trees!\n\n{:#?}",
        missing.len(),
        t.1.obligations.len(),
        missing
      );
    });
  });
}
