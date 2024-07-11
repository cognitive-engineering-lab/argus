// TODO: this is a very naive implementation, we can certainly make it more efficient.
pub struct And<I: Copy>(Vec<I>);
pub struct Dnf<I: Copy>(Vec<And<I>>);

impl<I: Copy> IntoIterator for And<I> {
  type Item = I;
  type IntoIter = std::vec::IntoIter<I>;

  fn into_iter(self) -> Self::IntoIter {
    self.0.into_iter()
  }
}

impl<I: Copy> And<I> {
  #[inline]
  pub fn iter(&self) -> impl Iterator<Item = &I> + '_ {
    self.0.iter()
  }

  fn distribute(&self, rhs: &Dnf<I>) -> Dnf<I> {
    Dnf(
      rhs
        .0
        .iter()
        .map(|And(rhs)| {
          And(self.0.iter().copied().chain(rhs.iter().copied()).collect())
        })
        .collect(),
    )
  }
}

impl<I: Copy> Dnf<I> {
  pub fn into_iter_conjuncts(self) -> impl Iterator<Item = And<I>> {
    self.0.into_iter()
  }

  pub fn and(vs: impl Iterator<Item = Self>) -> Option<Self> {
    vs.fold(None, |opt_lhs, rhs| match opt_lhs {
      None => Some(rhs),
      Some(lhs) => Self::distribute(lhs, rhs),
    })
  }

  pub fn or(vs: impl Iterator<Item = Self>) -> Option<Self> {
    let vs = vs.flat_map(|Self(v)| v).collect::<Vec<_>>();
    if vs.is_empty() {
      None
    } else {
      Some(Self(vs))
    }
  }

  #[allow(clippy::needless_pass_by_value)]
  fn distribute(self, other: Self) -> Option<Self> {
    Self::or(
      self
        .0
        .into_iter()
        .map(|conjunct| conjunct.distribute(&other)),
    )
  }

  #[inline]
  pub fn single(i: I) -> Self {
    Self(vec![And(vec![i])])
  }

  #[inline]
  pub fn default() -> Self {
    Self(vec![])
  }
}
