pub fn pick_selected<'a, T>(
  slice: impl IntoIterator<Item = T> + 'a,
  idxs: Vec<usize>,
) -> impl Iterator<Item = T> + 'a {
  slice
    .into_iter()
    .enumerate()
    .filter_map(move |(i, t)| idxs.contains(&i).then_some(t))
}
