import { ProofNodeIdx } from "@argus/common/bindings";
import _ from "lodash";
import React, { useContext } from "react";

import { TreeContext } from "./Context";
import { DirRecursive } from "./Directory";

const BottomUp = () => {
  const tree = useContext(TreeContext)!;
  const leaves = _.map(tree.errorNodes(), leaf => {
    let curr: ProofNodeIdx | undefined = leaf;
    while (curr !== undefined && tree.node(curr).type !== "goal") {
      curr = tree.parent(curr);
    }
    return curr;
  });

  const getParent = (idx: ProofNodeIdx) => {
    let p = tree.parent(idx);
    return p !== undefined ? [p] : [];
  };

  return _.map(leaves, (leaf, i) =>
    leaf === undefined ? (
      ""
    ) : (
      <DirRecursive
        key={i}
        level={[leaf]}
        getNext={getParent}
        styleEdges={false}
      />
    )
  );
};

export default BottomUp;
