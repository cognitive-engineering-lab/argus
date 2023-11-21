import { SerializedTree } from "@argus/common/types";
import _ from "lodash";
import React from "react";
import ReactJson from "react-json-view";

import "./TreeApp.css";
import { ActiveContext, ActiveState, TreeContext } from "./Context";
import Sidebar from "./Sidebar";
import Tabs from "./Tabs";
import TreeArea from "./TreeArea";
import TreeTopDown from "./TreeTopDown";
import TreeBottomUp from "./TreeBottomup";

// TODO: don't really need this.
type UnderlyingTree = SerializedTree[];

function getAttempt(tree: UnderlyingTree) {
  return _.maxBy(tree, (attempt: SerializedTree) => {
    return attempt.nodes.length;
  })!;
}

let TreeApp = ({ tree }: { tree: UnderlyingTree }) => {
  console.log("Initial data", tree);
  let attempt = getAttempt(tree);

  let tabs: [string, React.ReactNode][] = [
    ["Graph", <TreeArea tree={attempt} />],
    ["TopDown", <TreeTopDown tree={attempt}/>],
    ["BottomUp", <TreeBottomUp tree={attempt}/>],
    ["JSON", <ReactJson src={attempt} />],
  ];

  return (
    <TreeContext.Provider value={attempt}>
      <ActiveContext.Provider value={new ActiveState()}>
        <div className="App">
          <Tabs components={tabs} />
        </div>
      </ActiveContext.Provider>
    </TreeContext.Provider>
  );
};

export default TreeApp;
