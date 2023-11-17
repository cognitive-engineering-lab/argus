import { SerializedTree } from "@argus/common/types";
import _ from "lodash";
import React from "react";

import { DirRecursive } from "./Directory";

let TreeTopDown = ({ tree }: { tree: SerializedTree }) => {
  const getChildren = (tree: SerializedTree, idx: number) => {
    return _.reject(tree.topology.children[idx] || [], idx =>
      tree.unnecessaryRoots.includes(idx)
    );
  };
  return <DirRecursive level={[tree.root]} getNext={getChildren} styleEdges={true} />;
};

export default TreeTopDown;
