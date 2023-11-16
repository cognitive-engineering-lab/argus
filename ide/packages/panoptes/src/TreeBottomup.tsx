import { SerializedTree } from "@argus/common/types";
import _ from "lodash";
import React from "react";

import { DirRecursive } from "./Directory";
import { pathToRoot } from "./utilities";

let TreeBottomUp = ({ tree }: { tree: SerializedTree }) => {
  let allLeaves = _.map(tree.error_leaves, idx => tree.topology.parent[idx]!);
  let leaves = _.filter(allLeaves, idx => {
    let ancestors = pathToRoot(tree, idx);
    // Filter out leaves that are decendents of unnecessary roots
    return _.some(ancestors.path, idx => tree.unnecessary_roots.includes(idx));
  });

  const getParent = (tree: SerializedTree, idx: number) => {
    let p = tree.topology.parent[idx];
    return p != undefined ? [p] : [];
  };

  return (
    <>
      {_.map(leaves, (leaf, i) => {
        return <DirRecursive key={i} level={[leaf]} getNext={getParent} />;
      })}
    </>
  );
};

export default TreeBottomUp;
