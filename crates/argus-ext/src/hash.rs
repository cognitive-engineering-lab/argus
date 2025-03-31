use rustc_hashes::Hash64;
use rustc_data_structures::stable_hasher::{HashStable, StableHasher};
use rustc_middle::ty::{TyCtxt, TypeFoldable};
use rustc_query_system::ich::StableHashingContext;

pub trait StableHash<'__ctx, 'tcx>:
  HashStable<StableHashingContext<'__ctx>>
{
  fn stable_hash(
    self,
    infcx: &TyCtxt<'tcx>,
    ctx: &mut StableHashingContext<'__ctx>,
  ) -> Hash64;
}

impl<'__ctx, 'tcx, T> StableHash<'__ctx, 'tcx> for T
where
  T: HashStable<StableHashingContext<'__ctx>>,
  T: TypeFoldable<TyCtxt<'tcx>>,
{
  fn stable_hash(
    self,
    tcx: &TyCtxt<'tcx>,
    ctx: &mut StableHashingContext<'__ctx>,
  ) -> Hash64 {
    let mut h = StableHasher::new();
    let sans_regions = tcx.erase_regions(self);
    sans_regions.hash_stable(ctx, &mut h);
    h.finish()
  }
}
