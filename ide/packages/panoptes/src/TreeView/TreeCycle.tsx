import { ProofCycle } from "@argus/common/bindings";
import { TreeAppContext } from "@argus/common/context";
import _ from "lodash";
import React, { useContext } from "react";

import { DirRecursive } from "./Directory";

const TreeCycle = ({ path }: { path: ProofCycle }) => {
  const tree = useContext(TreeAppContext.TreeContext)!;

  const getChildren = (idx: number) => {
    const found = _.indexOf(path, idx);
    if (found < 0) {
      return [];
    }
    const next = path[found + 1];
    return next === undefined ? [] : [next];
  };

  return (
    <TreeAppContext.TreeRenderContext.Provider value={{ styleEdges: true }}>
      <DirRecursive level={[tree.root]} getNext={getChildren} />
    </TreeAppContext.TreeRenderContext.Provider>
  );
};

export default TreeCycle;
