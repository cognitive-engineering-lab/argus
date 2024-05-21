import { SerializedTree } from "@argus/common/bindings";
import {
  VSCodePanelTab,
  VSCodePanelView,
  VSCodePanels,
} from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React, { useContext } from "react";

import { AppContext, TreeAppContext } from "../utilities/context";
import BottomUp from "./BottomUp";
import TopDown from "./TopDown";
import "./TreeApp.css";
import TreeCycle from "./TreeCycle";
import TreeInfo from "./TreeInfo";

const TreeApp = ({
  tree,
  showHidden = false,
}: {
  tree: SerializedTree | undefined;
  showHidden?: boolean;
}) => {
  const evalMode = useContext(AppContext.ConfigurationContext)!.evalMode;
  // FIXME: this shouldn't ever happen, if a properly hashed
  // value is sent and returned. I need to think more about how to handle
  // when we want to display "non-traditional" obligations.
  const ErrorMessage = () => (
    <div className="Error">
      <p>Whoops! Something went wrong:</p>
      <pre>No debug information found.</pre>
    </div>
  );

  if (tree === undefined) {
    console.error("Returned tree `undefined`");
    return <ErrorMessage />;
  }

  const treeInfo = TreeInfo.new(tree, showHidden);
  if (treeInfo === undefined) {
    console.error("Failed to create tree view");
    return <ErrorMessage />;
  }

  const tabs: [string, React.FC][] = [["Top Down", TopDown]];

  if (treeInfo.errorLeaves().length > 0) {
    tabs.unshift(["Bottom Up", BottomUp]);
  }

  // HACK: we shouldn't test for eval mode here but Playwright is off on the button click.
  if (tree.cycle !== undefined && evalMode === "release") {
    // FIXME: why do I need the '!' here? - - - - - --------  VVVVVVVV
    tabs.unshift(["Cycle Detected", () => <TreeCycle path={tree.cycle!} />]);
  }

  return (
    <TreeAppContext.TreeContext.Provider value={treeInfo}>
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
    </TreeAppContext.TreeContext.Provider>
  );
};

export default TreeApp;
