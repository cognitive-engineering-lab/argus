#![feature(
    rustc_private,
    trait_alias,
    never_type, // proof tree visitor
    min_specialization, // for rustc_index
    let_chains
)]

extern crate rustc_data_structures;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_hir_analysis;
extern crate rustc_hir_typeck;
extern crate rustc_infer;
extern crate rustc_macros;
extern crate rustc_middle;
extern crate rustc_query_system;
extern crate rustc_session;
extern crate rustc_serialize;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_trait_selection;
extern crate rustc_type_ir;

pub mod analysis;
pub mod proof_tree;
// pub mod serialize;
pub mod ty;

#[derive(Debug)]
pub struct Target {
  data: u64,
}

pub trait ToTarget {
  fn to_target(self) -> Target;
}

impl ToTarget for u64 {
  fn to_target(self) -> Target {
    Target {
      data: self,
    }
  }
}

// FIXME: this shouldn't currently be used, because we now rely on
// the serialization of rustc types, I need to update the TS-RS
// generation.
#[cfg(test)]
mod tests {
    use ts_rs::TS;
    use crate::proof_tree;
    use rustc_utils::source_map::{range, filename};

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
          proof_tree::Obligation,
          proof_tree::TreeTopology<proof_tree::ProofNodeIdx>,

          // From rustc_utils
          range::CharRange,
          range::CharPos,
          filename::FilenameIndex,
        }
    }
}
