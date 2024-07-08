import type { SetHeuristic } from "@argus/common/bindings";
import { AppContext, TreeAppContext } from "@argus/common/context";
import _ from "lodash";
import React, { useContext } from "react";

import { BottomUpImpersonator, invertViewWithRoots } from "./BottomUp";

export function sortedSubsets(sets: SetHeuristic[]) {
  return _.sortBy(sets, sets => sets.momentum / sets.velocity);
}

// FIXME: we need to present the sets together in a conjunct, instead of the flat list.
// The flat list is currently the best way to get evaluation metrics.
const FailedSubsets = () => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const evaluationMode =
    useContext(AppContext.ConfigurationContext)?.evalMode ?? "release";

  const flattenSets = (sets: SetHeuristic[]) =>
    _.flatten(
      _.map(sets, h =>
        invertViewWithRoots(
          _.map(h.goals, g => g.idx),
          tree
        )
      )
    );

  const sets = sortedSubsets(tree.failedSets);
  const suggestedPredicates = flattenSets(_.slice(sets, 0, 3));
  const others = flattenSets(_.slice(sets, 3));

  return (
    <BottomUpImpersonator
      recommended={suggestedPredicates}
      others={others}
      mode={evaluationMode}
    />
  );
};

export default FailedSubsets;