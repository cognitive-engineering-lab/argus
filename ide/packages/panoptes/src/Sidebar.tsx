import { SerializedTree } from "@argus/common/types";
import { trace } from "mobx";
import { observer } from "mobx-react";
import React, { createContext, useContext } from "react";
// @ts-ignore
import ReactJson from "react-json-view";

import { ActiveContext, TreeContext } from "./Context";
import { nodeContent } from "./utilities";
import "./Sidebar.css";

const NodeInfo = observer(() => {
  trace(true);
  const currentNode = useContext(ActiveContext);
  const currentTree = useContext(TreeContext);
  const activeNode = currentNode?.getActiveNode();

  const nodeJson = (idx: number, tree: SerializedTree) => {
    return nodeContent(tree.nodes[idx]!);
  };

  const innerJson =
    activeNode == null || currentTree == null ? (
      <></>
    ) : (
      <span className="goal">{nodeJson(activeNode, currentTree)}</span>
    );

  return (
    <div className="NodeInfo">
      <h2>Node Info!</h2>{" "}
      {innerJson}
    </div>
  );
});

const EdgeInfo = observer(() => {
  trace(true);
  const currentNode = useContext(ActiveContext);
  const activeNode = currentNode?.getActiveNode();
  return (
    <div className="EdgeInfo">
      <h2>Edge Info!</h2>
      {activeNode != null && <span>Node is active {activeNode}</span>}
    </div>
  );
});

const Sidebar = observer(() => {
  return (
    <>
      <NodeInfo />
      <EdgeInfo />
    </>
  );
});

export default Sidebar;
