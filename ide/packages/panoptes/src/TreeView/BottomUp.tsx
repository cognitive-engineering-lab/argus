import { ProofNodeIdx } from "@argus/common/bindings";
import _ from "lodash";
import React, { useContext } from "react";

import { TreeContext } from "./Context";
import { CollapsibleElement, DirRecursive } from "./Directory";

const BottomUp = () => {
  const tree = useContext(TreeContext)!;
  const getParent = (idx: ProofNodeIdx) => {
    let p = tree.parent(idx);
    return p !== undefined ? [p] : [];
  };

  const leaves = _.map(tree.errorNodesRecommendedOrder(), leaf => {
    let curr: ProofNodeIdx | undefined = leaf;
    while (curr !== undefined && !("Goal" in tree.node(curr))) {
      curr = tree.parent(curr);
    }
    return curr;
  });

  // The "Argus recommended" errors are shown expanded, and the
  // "others" are collapsed. Argus recommended errors are the ones
  // that failed or are ambiguous with a concrete type on the LHS.
  const [argusRecommended, others] = _.partition(_.compact(leaves), leaf => {
    const node = tree.node(leaf);
    if ("Goal" in node) {
      const goal = node.Goal.data;
      return (
        goal.result === "no" ||
        goal.result === "maybe-overflow" ||
        !goal.isLhsTyVar
      );
    } else {
      // Leaves should only be goals...
      return false;
    }
  });

  const LeafElement = ({ leaf }: { leaf: ProofNodeIdx }) => {
    return (
      <DirRecursive level={[leaf]} getNext={getParent} styleEdges={false} />
    );
  };

  const recommended = _.map(argusRecommended, (leaf, i) => (
    <LeafElement key={i} leaf={leaf} />
  ));

  const fallbacks =
    others.length === 0 ? null : (
      <CollapsibleElement
        info={<span>Other failures ...</span>}
        Children={() => (
          <>
            {_.map(others, (leaf, i) => (
              <LeafElement key={i} leaf={leaf} />
            ))}
          </>
        )}
      />
    );

  return (
    <>
      {recommended}
      {fallbacks}
    </>
  );
};

export default BottomUp;
