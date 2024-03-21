import { ProofCycle } from "@argus/common/bindings";
import _ from "lodash";
import React, { useContext } from "react";

import { TreeContext } from "./Context";
import { DirRecursive, TreeRenderContext } from "./Directory";

const TreeCycle = ({ path }: { path: ProofCycle }) => {
  const tree = useContext(TreeContext)!;

  const getChildren = (idx: number) => {
    const found = _.indexOf(path, idx);
    if (found < 0) {
      return [];
    }
    const next = path[found + 1];
    return next === undefined ? [] : [next];
  };

  return (
    <TreeRenderContext.Provider value={{ styleEdges: true }}>
      <DirRecursive level={[tree.root]} getNext={getChildren} />
    </TreeRenderContext.Provider>
  );
};

export default TreeCycle;
