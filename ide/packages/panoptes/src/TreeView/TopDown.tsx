import { ProofNodeIdx } from "@argus/common/bindings";
import _ from "lodash";
import React, { useContext } from "react";

import { TreeContext } from "./Context";
import { DirRecursive } from "./Directory";

const TopDown = () => {
  const tree = useContext(TreeContext)!;
  const getChildren = (idx: ProofNodeIdx) => {
    return _.reject(tree.topology.children[idx] || [], idx =>
      tree.unnecessaryRoots.includes(idx)
    );
  };
  return (
    <DirRecursive level={[tree.root]} getNext={getChildren} styleEdges={true} />
  );
};

export default TopDown;
