import { SetHeuristic } from "@argus/common/bindings";
import _ from "lodash";
import React, { useContext } from "react";

import { AppContext, TreeAppContext } from "../utilities/context";
import { BottomUpImpersonator, invertViewWithRoots } from "./BottomUp";

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
  const suggestedPredicates = flattenSets(_.slice(tree.failedSets, 0, 3));
  const others = flattenSets(_.slice(tree.failedSets, 3));

  return (
    <BottomUpImpersonator
      recommended={suggestedPredicates}
      others={others}
      mode={evaluationMode}
    />
  );
};

export default FailedSubsets;
