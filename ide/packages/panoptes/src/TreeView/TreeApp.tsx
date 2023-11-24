import { SerializedTree } from "@argus/common/types";
import {
  VSCodePanelTab,
  VSCodePanelView,
  VSCodePanels,
} from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React from "react";
import ReactJson from "react-json-view";

import BottomUp from "./BottomUp";
import TreeArea from "./Graph";
import TopDown from "./TopDown";
import "./TreeApp.css";
import { ActiveContext, ActiveState, TreeContext } from "./context";

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
    ["TopDown", <TopDown tree={attempt} />],
    ["BottomUp", <BottomUp tree={attempt} />],
    ["JSON", <ReactJson src={attempt} />],
  ];

  return (
    <TreeContext.Provider value={attempt}>
      <ActiveContext.Provider value={new ActiveState()}>
        <div className="App">
          <VSCodePanels>
            {_.map(tabs, ([name, _], idx) => {
              return (
                <VSCodePanelTab key={idx} id={`tab-${idx}`}>
                  {name}
                </VSCodePanelTab>
              );
            })}
            {_.map(tabs, ([_, component], idx) => {
              return (
                <VSCodePanelView key={idx} id={`tab-${idx}`}>
                  {component}
                </VSCodePanelView>
              );
            })}
          </VSCodePanels>
        </div>
      </ActiveContext.Provider>
    </TreeContext.Provider>
  );
};

export default TreeApp;
