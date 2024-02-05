import { ProofNodeIdx } from "@argus/common/bindings";
import _ from "lodash";
import React, { useContext } from "react";

import { TreeContext } from "./Context";
import { DirRecursive } from "./Directory";

const BottomUp = () => {
  const tree = useContext(TreeContext)!;
  const allLeaves = _.map(tree.errorLeaves, idx => tree.topology.parent[idx]!);
  // let leaves = _.filter(allLeaves, idx => {
  //   let ancestors = pathToRoot(tree, idx);
  //   // Filter out leaves that are decendents of unnecessary roots
  //   return _.some(ancestors.path, idx => tree.unnecessaryRoots.includes(idx));
  // });
  // TODO(gavinleroy): we need to filter nodes that are decendents of unnecessary roots.
  const leaves = allLeaves;

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
