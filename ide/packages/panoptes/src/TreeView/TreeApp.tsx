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
import { TreeContext } from "./Context";
import TopDown from "./TopDown";
import "./TreeApp.css";
import TreeCycle from "./TreeCycle";
import TreeInfo from "./TreeInfo";

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

  const treeInfo = new TreeInfo(tree);

  let tabs: [string, React.FC][] = [
    ["Bottom Up", BottomUp],
    ["Top Down", TopDown],
    ["JSON", () => <ReactJson src={tree} />],
  ];

  if (tree.cycle !== undefined) {
    // FIXME: why do I need the '!' here? - - - - - --------  VVVVVVVV
    tabs.unshift(["Cycle Detected", () => <TreeCycle path={tree.cycle!} />]);
  }

  return (
    <TreeContext.Provider value={treeInfo}>
      <div className="App">
        <VSCodePanels>
          {_.map(tabs, ([name, _], idx) => (
            <VSCodePanelTab key={idx} id={`tab-${idx}`}>
              {name}
            </VSCodePanelTab>
          ))}
          {_.map(tabs, ([_, Component], idx) => (
            <VSCodePanelView key={idx} id={`tab-${idx}`}>
              <Component />
            </VSCodePanelView>
          ))}
        </VSCodePanels>
      </div>
    </TreeContext.Provider>
  );
};

export default TreeApp;
