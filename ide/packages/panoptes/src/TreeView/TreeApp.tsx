import { SerializedTree } from "@argus/common/bindings";
import {
  VSCodePanelTab,
  VSCodePanelView,
  VSCodePanels,
} from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React from "react";
import ReactJson from "react-json-view";

import BottomUp from "./BottomUp";
import { ActiveContext, ActiveState, TreeContext } from "./Context";
import TreeArea from "./Graph";
import TopDown from "./TopDown";
import "./TreeApp.css";

const TreeApp = ({ tree }: { tree: SerializedTree | undefined }) => {
  // FIXME: this shouldn't ever happen, if a properly hashed
  // value is sent and returned. I need to think more about how to handle
  // when we want to display "non-traditional" obligations.
  if (tree === undefined) {
    return (
      <div className="Error">
        <p>Whoops! Something went wrong:</p>
        <pre>No debug information found.</pre>
      </div>
    );
  }

  let tabs: [string, React.ReactNode][] = [
    // ["Graph", <TreeArea tree={attempt} />],
    ["BottomUp", <BottomUp tree={tree} />],
    ["TopDown", <TopDown tree={tree} />],
    ["JSON", <ReactJson src={tree} />],
  ];

  return (
    <TreeContext.Provider value={tree}>
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
