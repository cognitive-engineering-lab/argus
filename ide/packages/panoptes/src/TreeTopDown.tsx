import { SerializedTree } from "@argus/common/types";
import React from "react";

import { DirRecursive } from "./Directory";


let TreeTopDown = ({ tree }: { tree: SerializedTree }) => {
    const getChildren = (tree: SerializedTree, idx: number) => {
        return tree.topology.children[idx] || [];
    };
  return <DirRecursive level={[tree.descr.root]} getNext={getChildren}/>;
};

export default TreeTopDown;