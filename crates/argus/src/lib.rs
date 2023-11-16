#![feature(
    rustc_private,
    trait_alias,
    never_type,
    let_chains
)]

extern crate rustc_data_structures;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_hir_analysis;
extern crate rustc_hir_typeck;
extern crate rustc_infer;
extern crate rustc_middle;
extern crate rustc_trait_selection;
extern crate rustc_type_ir;

pub mod analysis;
pub mod proof_tree;

#[cfg(test)]
mod tests {
    use ts_rs::TS;
    use crate::proof_tree;

    macro_rules! ts {
      ($($ty:ty,)*) => {
        $({
          let error_msg = format!("Failed to export TS binding for type '{}'", stringify!($ty));
          <$ty as TS>::export().expect(error_msg.as_ref());
        })*
      };
    }

    #[test]
    fn export_bindings_all_tys() {
        ts! {
          proof_tree::SerializedTree,
          proof_tree::Node,
          proof_tree::TreeTopology<proof_tree::ProofNodeIdx>,
        }
    }
}
