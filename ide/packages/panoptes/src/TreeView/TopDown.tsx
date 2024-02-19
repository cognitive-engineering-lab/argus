import { Candidate, Goal, ProofNodeIdx } from "@argus/common/bindings";
import _ from "lodash";
import React, { useContext } from "react";

import { TreeContext } from "./Context";
import { DirRecursive } from "./Directory";

const TopDown = () => {
  const tree = useContext(TreeContext)!;

  const getGoalChildren = (kids: ProofNodeIdx[]) => {
    // Sort the candidates by the #infer vars / height of the tree
    return _.sortBy(kids, k => {
      const inferVars = tree.inferVars(k);
      const height = tree.maxHeigh(k);
      return inferVars / height;
    });
  };

  const getCandidateChildren = (kids: ProofNodeIdx[]) => {
    return _.sortBy(kids, k => {
      switch (tree.result(k) ?? "yes") {
        case "no":
          return 0;
        case "maybe-overflow":
          return 1;
        case "maybe-ambiguity":
          return 2;
        case "yes":
          return 3;
      }
    });
  };

  const getChildren = (idx: ProofNodeIdx) => {
    const node = tree.node(idx);
    const kids = tree.children(idx);
    if ("Goal" in node) {
      return getGoalChildren(kids);
    } else if ("Candidate" in node) {
      return getCandidateChildren(kids);
    } else {
      return [];
    }
  };
  return (
    <DirRecursive level={[tree.root]} getNext={getChildren} styleEdges={true} />
  );
};

export default TopDown;
