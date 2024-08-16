use std::{
  cmp::{Eq, PartialEq},
  hash::Hash,
};

use argus_ext::{
  infer::InferCtxtExt, ty::VarCounterExt, utils::SpanExt as ArgusSpanExt,
};
use argus_ser as ser;
use argus_ser::interner::Interner;
use index_vec::IndexVec;
use rustc_data_structures::stable_hasher::Hash64;
use rustc_hir::def_id::DefId;
use rustc_infer::infer::InferCtxt;
use rustc_trait_selection::{
  solve::inspect::{InspectCandidate, InspectGoal},
  traits::{
    solve,
    solve::{inspect::ProbeKind, CandidateSource},
  },
};

use super::*;
use crate::{
  ext::InferCtxtExt as InferCtxtExt_, tls,
  types::intermediate::EvaluationResult,
};

pub struct Interners {
  goals: Interner<(Hash64, ResultIdx), GoalIdx, GoalData>,
  candidates: Interner<CanKey, CandidateIdx, CandidateData>,
  results: Interner<EvaluationResult, ResultIdx, ResultData>,
}

#[derive(PartialEq, Eq, Hash)]
enum CanKey {
  Impl(DefId),
  ParamEnv(usize),
  Str(&'static str),
}

impl Interners {
  pub fn default() -> Self {
    Self {
      goals: Interner::default(),
      candidates: Interner::default(),
      results: Interner::default(),
    }
  }

  pub fn take(
    self,
  ) -> (
    IndexVec<GoalIdx, GoalData>,
    IndexVec<CandidateIdx, CandidateData>,
    IndexVec<ResultIdx, ResultData>,
  ) {
    (
      self.goals.consume(),
      self.candidates.consume(),
      self.results.consume(),
    )
  }

  // NOTE: used in `test_utils`.
  #[allow(dead_code)]
  pub fn goal(&self, g: GoalIdx) -> &GoalData {
    self.goals.get_data(&g).expect("missing goal idx")
  }

  // NOTE: used in `test_utils`.
  #[allow(dead_code)]
  pub fn candidate(&self, c: CandidateIdx) -> &CandidateData {
    self.candidates.get_data(&c).expect("missing candidate idx")
  }

  pub fn mk_result_node(&mut self, result: EvaluationResult) -> Node {
    Node::Result(self.intern_result(result))
  }

  pub fn mk_goal_node(&mut self, goal: &InspectGoal) -> Node {
    let infcx = goal.infcx();
    let result_idx = self.intern_result(goal.result());
    let goal = goal.goal();
    let goal_idx = self.intern_goal(infcx, &goal, result_idx);
    Node::Goal(goal_idx)
  }

  pub fn mk_candidate_node(&mut self, candidate: &InspectCandidate) -> Node {
    let can_idx = match candidate.kind() {
      ProbeKind::Root { .. } => self.intern_can_string("root"),
      ProbeKind::NormalizedSelfTyAssembly => {
        self.intern_can_string("normalized-self-ty-asm")
      }
      ProbeKind::TryNormalizeNonRigid { .. } => {
        self.intern_can_string("try-normalize-non-rigid")
      }
      ProbeKind::UnsizeAssembly => self.intern_can_string("unsize-asm"),
      ProbeKind::UpcastProjectionCompatibility => {
        self.intern_can_string("upcase-proj-compat")
      }
      ProbeKind::TraitCandidate { source, .. } => match source {
        CandidateSource::CoherenceUnknowable => {
          self.intern_can_string("coherence-unknowable")
        }
        CandidateSource::BuiltinImpl(_built_impl) => {
          self.intern_can_string("builtin")
        }
        CandidateSource::AliasBound => self.intern_can_string("alias-bound"),
        // The only two we really care about.
        CandidateSource::ParamEnv(idx) => self.intern_can_param_env(idx),

        CandidateSource::Impl(def_id) => {
          self.intern_impl(candidate.goal().infcx(), def_id)
        }
      },
      ProbeKind::ShadowedEnvProbing => {
        self.intern_can_string("shadowed-env-probing")
      }
      ProbeKind::OpaqueTypeStorageLookup { .. } => {
        self.intern_can_string("opaque-type-storage-lookup")
      }
    };

    Node::Candidate(can_idx)
  }

  fn intern_result(&mut self, result: EvaluationResult) -> ResultIdx {
    if let Some(result_idx) = self.results.get_idx(&result) {
      return result_idx;
    }

    self.results.insert(result, ResultData(result))
  }

  fn intern_goal<'tcx>(
    &mut self,
    infcx: &InferCtxt<'tcx>,
    goal: &solve::Goal<'tcx, ty::Predicate<'tcx>>,
    result_idx: ResultIdx,
  ) -> GoalIdx {
    let goal = infcx.resolve_vars_if_possible(*goal);
    let hash = infcx.predicate_hash(&goal.predicate);
    let hash = (hash, result_idx);
    if let Some(goal_idx) = self.goals.get_idx(&hash) {
      return goal_idx;
    }

    let necessity = infcx.guess_predicate_necessity(&goal.predicate);
    let num_vars = goal.predicate.count_vars(infcx.tcx);
    let is_main_tv = goal.predicate.is_main_ty_var();
    let goal_value = tls::unsafe_access_interner(|ty_interner| {
      ser::to_value_expect(infcx, ty_interner, &ser::GoalPredicateDef(goal))
    });

    self.goals.insert(hash, GoalData {
      value: goal_value,
      necessity,
      num_vars,
      is_main_tv,
      result: result_idx,

      #[cfg(debug_assertions)]
      debug_comparison: format!("{:?}", goal.predicate.kind().skip_binder()),
    })
  }

  fn intern_can_string(&mut self, s: &'static str) -> CandidateIdx {
    if let Some(i) = self.candidates.get_idx(&CanKey::Str(s)) {
      return i;
    }

    self.candidates.insert(CanKey::Str(s), s.into())
  }

  fn intern_can_param_env(&mut self, idx: usize) -> CandidateIdx {
    if let Some(i) = self.candidates.get_idx(&CanKey::ParamEnv(idx)) {
      return i;
    }

    self
      .candidates
      .insert(CanKey::ParamEnv(idx), CandidateData::ParamEnv(idx))
  }

  pub(super) fn intern_impl(
    &mut self,
    infcx: &InferCtxt,
    def_id: DefId,
  ) -> CandidateIdx {
    if let Some(i) = self.candidates.get_idx(&CanKey::Impl(def_id)) {
      return i;
    }

    // First, try to get an impl header from the def_id ty
    if let Some(header) = ser::get_opt_impl_header(infcx.tcx, def_id) {
      return self.candidates.insert(
        CanKey::Impl(def_id),
        CandidateData::new_impl_header(
          infcx,
          &header,
          infcx.tcx.is_user_visible_dep(def_id.krate),
        ),
      );
    }

    // Second, try to get the span of the impl or just default to a fallback.
    let string = infcx.tcx.span_of_impl(def_id).map_or_else(
      |symb| format!("foreign impl from: {}", symb.as_str()),
      |sp| sp.sanitized_snippet(infcx.tcx.sess.source_map()),
    );

    self.candidates.insert_no_key(CandidateData::from(string))
  }
}
