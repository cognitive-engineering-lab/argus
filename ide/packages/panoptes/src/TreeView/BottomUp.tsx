import { SerializedTree } from "@argus/common/bindings";
import _ from "lodash";
import React from "react";

import { DirRecursive } from "./Directory";

let BottomUp = ({ tree }: { tree: SerializedTree }) => {
  let allLeaves = _.map(tree.errorLeaves, idx => tree.topology.parent[idx]!);
  // let leaves = _.filter(allLeaves, idx => {
  //   let ancestors = pathToRoot(tree, idx);
  //   // Filter out leaves that are decendents of unnecessary roots
  //   return _.some(ancestors.path, idx => tree.unnecessaryRoots.includes(idx));
  // });
  // TODO(gavinleroy): we need to filter nodes that are decendents of unnecessary roots.
  let leaves = allLeaves;

  const getParent = (tree: SerializedTree, idx: number) => {
    let p = tree.topology.parent[idx];
    return p != undefined ? [p] : [];
  };

  return (
    <>
      {_.map(leaves, (leaf, i) => {
        return (
          <DirRecursive
            key={i}
            level={[leaf]}
            getNext={getParent}
            styleEdges={false}
          />
        );
      })}
    </>
  );
};

export default BottomUp;
