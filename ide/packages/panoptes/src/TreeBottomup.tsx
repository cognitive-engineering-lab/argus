import _ from "lodash";

import { SerializedTree } from "@argus/common/types";
import React from "react";
import { toRoot } from "./utilities";
import { DirRecursive } from "./Directory";


let TreeBottomUp = ({ tree }: { tree: SerializedTree }) => {
    let leaves = _.map(tree.error_leaves, idx => tree.topology.parent[idx]!);
    const getParent = (tree: SerializedTree, idx: number) => {
        let p = tree.topology.parent[idx];
        return p != undefined ? [p] : [];
    };

  return (
    <>
    {_.map(leaves, (leaf, i) => {
        return <DirRecursive key={i} level={[leaf]} getNext={getParent}/>;
    })}
    </>
  );
}

export default TreeBottomUp;