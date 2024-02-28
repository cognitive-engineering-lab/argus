use std::fmt::Debug;

use argus_lib::test_utils as tu;

fn print_first_diff<T: Debug>(arr: &[T]) {
  for w in arr.windows(2) {
    let s0 = format!("{:?}", &w[0]);
    let s1 = format!("{:?}", &w[1]);
    text_diff::assert_diff(&s0, &s1, "", 0);
  }
}

fn is_all_same<T: PartialEq>(arr: &[T]) -> bool {
  arr.windows(2).all(|w| w[0] == w[1])
}

#[test_log::test]
fn stability() {
  tu::run_in_dir("contained", |path| {
    let iterations = 10;
    let mut output_for_path = vec![];

    for _ in 0 .. iterations {
      let mut hashes = vec![];
      tu::test_obligations_no_crash(path, |_, obligations| {
        let body_hashes = obligations
          .obligations
          .into_iter()
          .map(|o| o.hash)
          .collect::<Vec<_>>();
        hashes.extend(body_hashes);
      });
      output_for_path.push(hashes);
    }

    assert!(
      is_all_same(&output_for_path),
      "Output for path was not stable {}",
      {
        print_first_diff(&output_for_path);
        "^^ DIFF ^^"
      },
    );
  });
}
