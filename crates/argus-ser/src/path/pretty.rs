//! Implementations from `rustc_middle::ty::print::pretty`
//
//! This code was modified in the following ways, ideally,
//! there could be a solution upstreamed into rustc, but that
//! seems like a pipe dream.
//! 1. All methods return the `Ok` value of the Result.
//! 2. Instead of writing to a buffer, we keep a structured
//!    version of path segments in the `segments` field.
//!    This is then serialized and "pretty printed" in the ide.

use default::PathBuilderDefault;
use rustc_hir::{
  def_id::{CrateNum, DefId, LOCAL_CRATE},
  definitions::{DefPathData, DefPathDataName, DisambiguatedDefPathData},
};
use rustc_middle::ty::{self, *};
use rustc_span::symbol::Ident;
use rustc_utils::source_map::range::CharRange;

use super::*;
use crate::ty::TraitRefPrintOnlyTraitPathDef;

impl<'tcx> PathBuilder<'tcx> {
  pub fn print_def_path(
    &mut self,
    def_id: DefId,
    args: &'tcx [GenericArg<'tcx>],
  ) {
    // CHANGE
    // No longer trying to print a trimmed or "visible" def path.

    let key = self.tcx().def_key(def_id);
    if let DefPathData::Impl = key.disambiguated_data.data {
      // Always use types for non-local impls, where types are always
      // available, and filename/line-number is mostly uninteresting.
      let use_types = !def_id.is_local() || {
        // Otherwise, use filename/line-number if forced.
        let force_no_types = with_forced_impl_filename_line();
        !force_no_types
      };

      if !use_types {
        // If no type info is available, fall back to
        // pretty printing some span information. This should
        // only occur very early in the compiler pipeline.
        let parent_def_id = DefId {
          index: key.parent.unwrap(),
          ..def_id
        };
        let span = self.tcx().def_span(def_id);

        self.print_def_path(parent_def_id, &[]);

        // HACK(eddyb) copy of `path_append` to avoid
        // constructing a `DisambiguatedDefPathData`.
        if !self.empty_path {
          self.segments.push(PathSegment::Colons);
        }

        // CHANGE: write!(
        //     self,
        //     "<impl at {}>",
        //     // This may end up in stderr diagnostics but it may also be emitted
        //     // into MIR. Hence we use the remapped path if available
        //     self.tcx.sess.source_map().span_to_embeddable_string(span)
        // )?;
        let impl_range =
          CharRange::from_span(span, self.tcx().sess.source_map())
            .expect("impl span not converted to `CharRange`");
        self
          .segments
          .push(PathSegment::AnonImpl { range: impl_range });

        self.empty_path = false;
        return;
      }
    }

    self.default_print_def_path(def_id, args)
  }

  pub fn path_crate(&mut self, cnum: CrateNum) {
    self.empty_path = true;
    if cnum == LOCAL_CRATE {
      if self.tcx().sess.at_least_rust_2018() {
        // We add the `crate::` keyword on Rust 2018, only when desired.
        if with_crate_prefix() {
          // CHANGE: write!(self, "{}", kw::Crate)?;
          self.segments.push(PathSegment::LocalCrate);
          self.empty_path = false;
        }
      }
    } else {
      // CHANGE: write!(self, "{}", self.tcx.crate_name(cnum))?;
      self
        .segments
        .push(PathSegment::unambiguous_name(self.tcx().crate_name(cnum)));
      self.empty_path = false;
    }
  }

  pub fn path_append(
    &mut self,
    print_prefix: impl FnOnce(&mut Self),
    disambiguated_data: &DisambiguatedDefPathData,
  ) {
    print_prefix(self);

    // Skip `::{{extern}}` blocks and `::{{constructor}}` on tuple/unit structs.
    if let DefPathData::ForeignMod | DefPathData::Ctor = disambiguated_data.data
    {
      return;
    }

    let name = disambiguated_data.data.name();
    if !self.empty_path {
      // CHANGE: write!(self, "::")?;
      self.segments.push(PathSegment::Colons);
    }

    if let DefPathDataName::Named(name) = name {
      if Ident::with_dummy_span(name).is_raw_guess() {
        // CHANGE: write!(self, "r#")?;
        self.segments.push(PathSegment::RawGuess);
      }
    }

    let verbose = self.should_print_verbose();
    // CHANGE: not printing on DisambiguatedDefData method.
    // dsamiguated_data.fmt_maybe_verbose(self, verbose);
    self.fmt_maybe_verbose(disambiguated_data, verbose);

    self.empty_path = false;
  }

  pub fn path_generic_args(
    &mut self,
    print_prefix: impl FnOnce(&mut Self),
    args: &[GenericArg<'tcx>],
  ) {
    print_prefix(self);

    if !args.is_empty() {
      if self.in_value {
        // CHANGE: write!(self, "::")?;
        self.segments.push(PathSegment::Colons);
      }
      self.segments.push(PathSegment::GenericArgumentList {
        entries: args.iter().copied().collect(),
      })
      // self.generic_delimiters(|cx| {
      //   #[derive(Serialize)]
      //   struct Wrapper<'a, 'tcx: 'a>(
      //     #[serde(with = "GenericArgDef")] &'a GenericArg<'tcx>,
      //   );
      //   cx.comma_sep(args.iter().map(Wrapper), CommaSeparatedKind::GenericArg);
      // });
    }
  }

  pub fn path_qualified(
    &mut self,
    self_ty: Ty<'tcx>,
    trait_ref: Option<ty::TraitRef<'tcx>>,
  ) {
    self.pretty_path_qualified(self_ty, trait_ref);
    self.empty_path = false;
  }

  pub fn pretty_path_qualified(
    &mut self,
    self_ty: Ty<'tcx>,
    trait_ref: Option<ty::TraitRef<'tcx>>,
  ) {
    if trait_ref.is_none() {
      // Inherent impls. Try to print `Foo::bar` for an inherent
      // impl on `Foo`, but fallback to `<Foo>::bar` if self-type is
      // anything other than a simple path.
      match self_ty.kind() {
        ty::Adt(..)
        | ty::Foreign(_)
        | ty::Bool
        | ty::Char
        | ty::Str
        | ty::Int(_)
        | ty::Uint(_)
        | ty::Float(_) => {
          // CHANGE: return self_ty.print(self);
          self.segments.push(PathSegment::Ty { ty: self_ty });
        }

        _ => {}
      }
    }

    self.generic_delimiters(|cx| {
      // CHANGE: define_scoped_cx!(cx);
      // p!(print(self_ty));
      // if let Some(trait_ref) = trait_ref {
      //     p!(" as ", print(trait_ref.print_only_trait_path()));
      // }
      cx.segments.push(PathSegment::Impl {
        path: trait_ref.map(|t| TraitRefPrintOnlyTraitPathDef::new(&t)),
        ty: self_ty,
        kind: ImplKind::As,
      });
    });
  }

  fn generic_delimiters(&mut self, f: impl FnOnce(&mut Self)) {
    // CHANGE: write!(self, "<")?;
    let before = std::mem::take(&mut self.segments);
    let was_in_value = std::mem::replace(&mut self.in_value, false);
    f(self);
    let after = std::mem::replace(&mut self.segments, before);
    self.in_value = was_in_value;
    self
      .segments
      .push(PathSegment::GenericDelimiters { inner: after });
    // CHANGE: write!(self, ">")?;
  }

  // NOTE: this was only used for generic argument lists, for now it's been removed,
  // in the future if we need a generic way of including commas you can bring it back.
  // /// Prints comma-separated elements.
  // fn comma_sep<T>(
  //   &mut self,
  //   elems: impl Iterator<Item = T>,
  //   kind: CommaSeparatedKind,
  // ) where
  //   T: Serialize,
  //   // T: Print<'tcx, Self>,
  // {
  //   // CHANGE:
  //   // if let Some(first) = elems.next() {
  //   //     // first.print(self)?;
  //   //     for elem in elems {
  //   //         // self.write_str(", ")?;
  //   //         // elem.print(self)?;
  //   //     }
  //   // }
  //   self.segments.push(PathSegment::CommaSeparated {
  //     entries: elems
  //       .map(|e| {
  //         serde_json::to_value(e)
  //           .expect("failed to serialize comma separated value")
  //       })
  //       .collect::<Vec<_>>(),
  //     kind,
  //   });
  // }

  pub fn path_append_impl(
    &mut self,
    print_prefix: impl FnOnce(&mut Self),
    _disambiguated_data: &DisambiguatedDefPathData,
    self_ty: Ty<'tcx>,
    trait_ref: Option<ty::TraitRef<'tcx>>,
  ) {
    self.pretty_path_append_impl(
      |cx| {
        print_prefix(cx);
        if !cx.empty_path {
          // CHANGE: write!(cx, "::")?;
          cx.segments.push(PathSegment::Colons);
        }
      },
      self_ty,
      trait_ref,
    );
    self.empty_path = false;
  }

  pub fn pretty_path_append_impl(
    &mut self,
    print_prefix: impl FnOnce(&mut Self),
    self_ty: Ty<'tcx>,
    trait_ref: Option<ty::TraitRef<'tcx>>,
  ) {
    print_prefix(self);

    log::trace!("pretty_path_append_impl {:?} {:?}", self_ty, trait_ref);

    self.generic_delimiters(|cx| {
      // CHANGE: define_scoped_cx!(cx);
      // p!("impl ");
      // if let Some(trait_ref) = trait_ref {
      //     p!(print(trait_ref.print_only_trait_path()), " for ");
      // }
      // p!(print(self_ty));
      cx.segments.push(PathSegment::Impl {
        ty: self_ty,
        path: trait_ref.map(|t| TraitRefPrintOnlyTraitPathDef::new(&t)),
        kind: ImplKind::For,
      });
    })
  }

  pub fn pretty_print_inherent_projection(
    &mut self,
    alias_ty: &ty::AliasTy<'tcx>,
  ) {
    let def_key = self.tcx().def_key(alias_ty.def_id);
    self.path_generic_args(
      |cx| {
        cx.path_append(
          |cx| cx.path_qualified(alias_ty.self_ty(), None),
          &def_key.disambiguated_data,
        );
      },
      &alias_ty.args[1 ..],
    );
  }
}
