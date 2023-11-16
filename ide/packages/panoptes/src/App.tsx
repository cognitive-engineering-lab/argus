import { UnderlyingTree } from "@argus/common";
import { QueryAttempt, SerializedTree } from "@argus/common/types";
import _ from "lodash";
import React from "react";
import ReactJson from "react-json-view";

import "./App.css";
import { ActiveContext, ActiveState, TreeContext } from "./Context";
import Sidebar from "./Sidebar";
import Tabs from "./Tabs";
import TreeArea from "./TreeArea";
import TreeTopDown from "./TreeTopDown";
import TreeBottomUp from "./TreeBottomup";

// HACK to get the topologies quickly :)
function getAttempt(tree: UnderlyingTree) {
  // const topos = _.map(tree, value => {
  //   if ("Required" in value.kind) {
  //     const secondToLast = value.kind.Required[value.kind.Required.length - 2];
  //     const attempt = (secondToLast || _.last(value.kind.Required))!;
  //     return attempt;
  //   } else {
  //     throw new Error("Not implemented");
  //   }
  // });

  return _.maxBy(tree, (attempt: SerializedTree) => {
    return attempt.nodes.length;
  })!;
}

let App = ({ tree }: { tree: UnderlyingTree }) => {
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

export default App;
