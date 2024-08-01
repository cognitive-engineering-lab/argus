use smallvec::{smallvec, SmallVec};

const MAX_CONJUNCTS: usize = 4;

// TODO: this is a very naive implementation, we can certainly make it more efficient.
#[derive(Clone)]
pub struct And<I: Copy>(SmallVec<[I; MAX_CONJUNCTS]>);

pub struct Dnf<I: Copy>(Vec<And<I>>);

impl<I: Copy> IntoIterator for And<I> {
  type Item = I;
  type IntoIter = smallvec::IntoIter<[I; MAX_CONJUNCTS]>;

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
  pub fn iter_conjuncts(&self) -> impl Iterator<Item = &And<I>> {
    self.0.iter()
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
    Self(vec![And(smallvec![i])])
  }

  #[inline]
  pub fn default() -> Self {
    Self(vec![])
  }
}
