//! Implementations from rustc_middle::ty::print::pretty
//
//! This code was modified in the following ways, ideally,
//! there could be a solution upstreamed into rustc, but that
//! seems like a pipe dream.
//! 1. All methods return the `Ok` value of the Result.
//! 2. Instead of writing to a buffer, we keep a structured
//!    version of path segments in the `segments` field.
//!    This is then serialized and "pretty printed" in the ide.

use default::PathBuilderDefault;
use log::debug;
use rustc_hir::{
  def::DefKind,
  def_id::{CrateNum, DefId, ModDefId, LOCAL_CRATE},
  definitions::{
    DefKey, DefPathData, DefPathDataName, DisambiguatedDefPathData,
  },
};
use rustc_middle::ty::{self, *};
use rustc_session::cstore::{ExternCrate, ExternCrateSource};
use rustc_span::symbol::{kw, Ident};
use rustc_utils::source_map::range::CharRange;
use serde::Serialize;

use super::*;

impl<'a, 'tcx: 'a, S: serde::Serializer> PathBuilder<'a, 'tcx, S> {
  pub fn print_def_path(
    &mut self,
    def_id: DefId,
    args: &'tcx [GenericArg<'tcx>],
  ) {
    if args.is_empty() {
      if self.try_print_trimmed_def_path(def_id) {
        return;
      }

      if self.try_print_visible_def_path(def_id) {
        return;
      }
    }

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

  pub fn try_print_trimmed_def_path(&mut self, def_id: DefId) -> bool {
    return false; // FIXME:(gavinleroy)

    // if with_forced_trimmed_paths() && self.force_print_trimmed_def_path(def_id)
    // {
    //   return true;
    // }
    // if self.tcx().sess.opts.unstable_opts.trim_diagnostic_paths
    //   && self.tcx().sess.opts.trimmed_def_paths
    //   && !with_no_trimmed_paths()
    //   && !with_crate_prefix()
    //   && let Some(symbol) = self.tcx().trimmed_def_paths(()).get(&def_id)
    // {
    //   // CHANGE: write!(self, "{}", Ident::with_dummy_span(*symbol))?;
    //   self.segments.push(PathSegment::unambiguous_name(*symbol));
    //   true
    // } else {
    //   false
    // }
  }

  /// Does the work of `try_print_visible_def_path`, building the
  /// full definition path recursively before attempting to
  /// post-process it into the valid and visible version that
  /// accounts for re-exports.
  ///
  /// This method should only be called by itself or
  /// `try_print_visible_def_path`.
  ///
  /// `callers` is a chain of visible_parent's leading to `def_id`,
  /// to support cycle detection during recursion.
  ///
  /// This method returns false if we can't print the visible path, so
  /// `print_def_path` can fall back on the item's real definition path.
  fn try_print_visible_def_path_recur(
    &mut self,
    def_id: DefId,
    callers: &mut Vec<DefId>,
  ) -> bool {
    debug!("try_print_visible_def_path: def_id={:?}", def_id);

    // If `def_id` is a direct or injected extern crate, return the
    // path to the crate followed by the path to the item within the crate.
    if let Some(cnum) = def_id.as_crate_root() {
      if cnum == LOCAL_CRATE {
        self.path_crate(cnum);
        return true;
      }

      // In local mode, when we encounter a crate other than
      // LOCAL_CRATE, execution proceeds in one of two ways:
      //
      // 1. For a direct dependency, where user added an
      //    `extern crate` manually, we put the `extern
      //    crate` as the parent. So you wind up with
      //    something relative to the current crate.
      // 2. For an extern inferred from a path or an indirect crate,
      //    where there is no explicit `extern crate`, we just prepend
      //    the crate name.
      match self.tcx().extern_crate(def_id) {
        Some(&ExternCrate {
          src,
          dependency_of,
          span,
          ..
        }) => match (src, dependency_of) {
          (ExternCrateSource::Extern(def_id), LOCAL_CRATE) => {
            // NOTE(eddyb) the only reason `span` might be dummy,
            // that we're aware of, is that it's the `std`/`core`
            // `extern crate` injected by default.
            // FIXME(eddyb) find something better to key this on,
            // or avoid ending up with `ExternCrateSource::Extern`,
            // for the injected `std`/`core`.
            if span.is_dummy() {
              self.path_crate(cnum);
              return true;
            }

            // Disable `try_print_trimmed_def_path` behavior within
            // the `print_def_path` call, to avoid infinite recursion
            // in cases where the `extern crate foo` has non-trivial
            // parents, e.g. it's nested in `impl foo::Trait for Bar`
            // (see also issues #55779 and #87932).
            with_no_visible_paths!(self.print_def_path(def_id, &[]));

            return true;
          }
          (ExternCrateSource::Path, LOCAL_CRATE) => {
            self.path_crate(cnum);
            return true;
          }
          _ => {}
        },
        None => {
          self.path_crate(cnum);
          return true;
        }
      }
    }

    if def_id.is_local() {
      return false;
    }

    let visible_parent_map = self.tcx().visible_parent_map(());

    let mut cur_def_key = self.tcx().def_key(def_id);
    debug!("try_print_visible_def_path: cur_def_key={:?}", cur_def_key);

    // For a constructor, we want the name of its parent rather than <unnamed>.
    if let DefPathData::Ctor = cur_def_key.disambiguated_data.data {
      let parent = DefId {
        krate: def_id.krate,
        index: cur_def_key
          .parent
          .expect("`DefPathData::Ctor` / `VariantData` missing a parent"),
      };

      cur_def_key = self.tcx().def_key(parent);
    }

    let Some(visible_parent) = visible_parent_map.get(&def_id).cloned() else {
      return false;
    };

    let actual_parent = self.tcx().opt_parent(def_id);
    debug!(
      "try_print_visible_def_path: visible_parent={:?} actual_parent={:?}",
      visible_parent, actual_parent,
    );

    let mut data = cur_def_key.disambiguated_data.data;
    debug!(
          "try_print_visible_def_path: data={:?} visible_parent={:?} actual_parent={:?}",
          data, visible_parent, actual_parent,
      );

    match data {
      // In order to output a path that could actually be imported (valid and visible),
      // we need to handle re-exports correctly.
      //
      // For example, take `std::os::unix::process::CommandExt`, this trait is actually
      // defined at `std::sys::unix::ext::process::CommandExt` (at time of writing).
      //
      // `std::os::unix` reexports the contents of `std::sys::unix::ext`. `std::sys` is
      // private so the "true" path to `CommandExt` isn't accessible.
      //
      // In this case, the `visible_parent_map` will look something like this:
      //
      // (child) -> (parent)
      // `std::sys::unix::ext::process::CommandExt` -> `std::sys::unix::ext::process`
      // `std::sys::unix::ext::process` -> `std::sys::unix::ext`
      // `std::sys::unix::ext` -> `std::os`
      //
      // This is correct, as the visible parent of `std::sys::unix::ext` is in fact
      // `std::os`.
      //
      // When printing the path to `CommandExt` and looking at the `cur_def_key` that
      // corresponds to `std::sys::unix::ext`, we would normally print `ext` and then go
      // to the parent - resulting in a mangled path like
      // `std::os::ext::process::CommandExt`.
      //
      // Instead, we must detect that there was a re-export and instead print `unix`
      // (which is the name `std::sys::unix::ext` was re-exported as in `std::os`). To
      // do this, we compare the parent of `std::sys::unix::ext` (`std::sys::unix`) with
      // the visible parent (`std::os`). If these do not match, then we iterate over
      // the children of the visible parent (as was done when computing
      // `visible_parent_map`), looking for the specific child we currently have and then
      // have access to the re-exported name.
      DefPathData::TypeNs(ref mut name)
        if Some(visible_parent) != actual_parent =>
      {
        // Item might be re-exported several times, but filter for the one
        // that's public and whose identifier isn't `_`.
        let reexport = self
          .tcx()
          // FIXME(typed_def_id): Further propagate ModDefId
          .module_children(ModDefId::new_unchecked(visible_parent))
          .iter()
          .filter(|child| child.res.opt_def_id() == Some(def_id))
          .find(|child| {
            child.vis.is_public() && child.ident.name != kw::Underscore
          })
          .map(|child| child.ident.name);

        if let Some(new_name) = reexport {
          *name = new_name;
        } else {
          // There is no name that is public and isn't `_`, so bail.
          return false;
        }
      }
      // Re-exported `extern crate` (#43189).
      DefPathData::CrateRoot => {
        data = DefPathData::TypeNs(self.tcx().crate_name(def_id.krate));
      }
      _ => {}
    }
    debug!("try_print_visible_def_path: data={:?}", data);

    if callers.contains(&visible_parent) {
      return false;
    }
    callers.push(visible_parent);
    // HACK(eddyb) this bypasses `path_append`'s prefix printing to avoid
    // knowing ahead of time whether the entire path will succeed or not.
    // To support printers that do not implement `PrettyPrinter`, a `Vec` or
    // linked list on the stack would need to be built, before any printing.
    match self.try_print_visible_def_path_recur(visible_parent, callers) {
      false => return false,
      true => {}
    }
    callers.pop();
    self.path_append(|_| {}, &DisambiguatedDefPathData {
      data,
      disambiguator: 0,
    });
    true
  }

  /// If possible, this returns a global path resolving to `def_id` that is visible
  /// from at least one local module, and returns `true`. If the crate defining `def_id` is
  /// declared with an `extern crate`, the path is guaranteed to use the `extern crate`.
  fn try_print_visible_def_path(&mut self, def_id: DefId) -> bool {
    if with_no_visible_paths() {
      return false;
    }

    let mut callers = Vec::new();
    self.try_print_visible_def_path_recur(def_id, &mut callers)
  }

  // Given a `DefId`, produce a short name. For types and traits, it prints *only* its name,
  // For associated items on traits it prints out the trait's name and the associated item's name.
  // For enum variants, if they have an unique name, then we only print the name, otherwise we
  // print the enum name and the variant name. Otherwise, we do not print anything and let the
  // caller use the `print_def_path` fallback.
  fn force_print_trimmed_def_path(&mut self, def_id: DefId) -> bool {
    return false; // FIXME:(gavin);

    let key = self.tcx().def_key(def_id);
    let visible_parent_map = self.tcx().visible_parent_map(());
    let kind = self.tcx().def_kind(def_id);

    let get_local_name = |this: &Self, name, def_id, key: DefKey| {
      if let Some(visible_parent) = visible_parent_map.get(&def_id)
        && let actual_parent = this.tcx().opt_parent(def_id)
        && let DefPathData::TypeNs(_) = key.disambiguated_data.data
        && Some(*visible_parent) != actual_parent
      {
        this
          .tcx()
          // FIXME(typed_def_id): Further propagate ModDefId
          .module_children(ModDefId::new_unchecked(*visible_parent))
          .iter()
          .filter(|child| child.res.opt_def_id() == Some(def_id))
          .find(|child| {
            child.vis.is_public() && child.ident.name != kw::Underscore
          })
          .map(|child| child.ident.name)
          .unwrap_or(name)
      } else {
        name
      }
    };
    // FIXME:(gavinleroy)
    // if let DefKind::Variant = kind
    //   && let Some(symbol) = self.tcx().trimmed_def_paths(()).get(&def_id)
    // {
    //   // If `Assoc` is unique, we don't want to talk about `Trait::Assoc`.
    //   // CHANGE: self.write_str(get_local_name(self, *symbol, def_id, key).as_str())?;
    //   self
    //     .segments
    //     .push(PathSegment::unambiguous_name(get_local_name(
    //       self, *symbol, def_id, key,
    //     )));
    //   return true;
    // }
    if let Some(symbol) = key.get_opt_name() {
      if let DefKind::AssocConst | DefKind::AssocFn | DefKind::AssocTy = kind
        && let Some(parent) = self.tcx().opt_parent(def_id)
        && let parent_key = self.tcx().def_key(parent)
        && let Some(symbol) = parent_key.get_opt_name()
      {
        // CHANGE: Trait
        // self.write_str(get_local_name(self, symbol, parent, parent_key).as_str())?;
        // self.write_str("::")?;
        self
          .segments
          .push(PathSegment::unambiguous_name(get_local_name(
            self, symbol, parent, parent_key,
          )));
        self.segments.push(PathSegment::Colons);
      } else if let DefKind::Variant = kind
        && let Some(parent) = self.tcx().opt_parent(def_id)
        && let parent_key = self.tcx().def_key(parent)
        && let Some(symbol) = parent_key.get_opt_name()
      {
        // CHANGE: Enum
        // For associated items and variants, we want the "full" path, namely, include
        // the parent type in the path. For example, `Iterator::Item`.
        // self.write_str(get_local_name(self, symbol, parent, parent_key).as_str())?;
        // self.write_str("::")?;
        self
          .segments
          .push(PathSegment::unambiguous_name(get_local_name(
            self, symbol, parent, parent_key,
          )));
        self.segments.push(PathSegment::Colons);
      } else if let DefKind::Struct
      | DefKind::Union
      | DefKind::Enum
      | DefKind::Trait
      | DefKind::TyAlias
      | DefKind::Fn
      | DefKind::Const
      | DefKind::Static(_) = kind
      {
      } else {
        // If not covered above, like for example items out of `impl` blocks, fallback.
        return false;
      }
      // CHANGE: self.write_str(get_local_name(self, symbol, def_id, key).as_str())?;
      self
        .segments
        .push(PathSegment::unambiguous_name(get_local_name(
          self, symbol, def_id, key,
        )));
      return true;
    }
    false
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
      self.generic_delimiters(|cx| {
        #[derive(Serialize)]
        struct Wrapper<'a, 'tcx: 'a>(
          #[serde(with = "GenericArgDef")] &'a GenericArg<'tcx>,
        );
        cx.comma_sep(
          args.into_iter().map(Wrapper),
          CommaSeparatedKind::GenericArg,
        )
      })
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
        path: trait_ref.map(|t| TraitRefPrintOnlyTraitPathDefWrapper(t)),
        ty: self_ty,
        kind: ImplKind::As,
      })
    })
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
      .push(PathSegment::GenericDelimiters { inner: after })
    // CHANGE: write!(self, ">")?;
  }

  /// Prints comma-separated elements.
  fn comma_sep<T>(
    &mut self,
    elems: impl Iterator<Item = T>,
    kind: CommaSeparatedKind,
  ) where
    T: Serialize,
    // T: Print<'tcx, Self>,
  {
    // CHANGE:
    // if let Some(first) = elems.next() {
    //     // first.print(self)?;
    //     for elem in elems {
    //         // self.write_str(", ")?;
    //         // elem.print(self)?;
    //     }
    // }
    self.segments.push(PathSegment::CommaSeparated {
      entries: elems
        .map(|e| {
          serde_json::to_value(e)
            .expect("failed to serialize comma separated value")
        })
        .collect::<Vec<_>>(),
      kind,
    });
  }

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

    log::debug!("pretty_path_append_impl {:?} {:?}", self_ty, trait_ref);

    self.generic_delimiters(|cx| {
      // CHANGE: define_scoped_cx!(cx);
      // p!("impl ");
      // if let Some(trait_ref) = trait_ref {
      //     p!(print(trait_ref.print_only_trait_path()), " for ");
      // }
      // p!(print(self_ty));
      cx.segments.push(PathSegment::Impl {
        ty: self_ty,
        path: trait_ref.map(|t| TraitRefPrintOnlyTraitPathDefWrapper(t)),
        kind: ImplKind::For,
      })
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
        )
      },
      &alias_ty.args[1 ..],
    )
  }
}
