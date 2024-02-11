import { ProofNodeIdx } from "@argus/common/bindings";
import _ from "lodash";
import React, { useContext } from "react";

import { TreeContext } from "./Context";
import { DirRecursive } from "./Directory";

const BottomUp = () => {
  const tree = useContext(TreeContext)!;
  const leaves = tree.errorLeaves;

  // TODO: start from the first non-leaf goal and go up the tree.

  const getParent = (idx: ProofNodeIdx) => {
    let p = tree.topology.parent[idx];
    return p != undefined ? [p] : [];
  };

  return _.map(leaves, (leaf, i) => (
    <DirRecursive
      key={i}
      level={[leaf]}
      getNext={getParent}
      styleEdges={false}
    />
  ));
};

export default BottomUp;
