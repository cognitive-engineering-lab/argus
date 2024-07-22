import TreeInfo from "@argus/common/TreeInfo";
import type { SerializedTree } from "@argus/common/bindings";
import { AppContext, TreeAppContext } from "@argus/common/context";
import { TyCtxt } from "@argus/print/context";
import {
  VSCodePanelTab,
  VSCodePanelView,
  VSCodePanels
} from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React, { useContext } from "react";

import Erotisi from "./Erotisi";
import FailedSubsets from "./Subsets";
import TopDown from "./TopDown";
import "./TreeApp.css";
import TreeCycle from "./TreeCycle";

const TreeApp = ({
  tree,
  showHidden = false
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

  const internedTys = tree.tys;
  const treeInfo = TreeInfo.new(tree, showHidden);
  if (treeInfo === undefined) {
    console.error("Failed to create tree view");
    return <ErrorMessage />;
  }

  const tabs: [string, React.FC][] = [["Top Down", TopDown]];

  if (treeInfo.errorLeaves().length > 0) {
    // Unshift to place this first
    tabs.unshift(["Bottom Up", FailedSubsets]);

    // Push to place this last
    tabs.push(["Help Me", Erotisi]);
  }

  // HACK: we shouldn't test for eval mode here but Playwright is off on the button click.
  if (tree.cycle !== undefined && evalMode === "release") {
    // FIXME: why do I need the '!' here? - - - - - --------  VVVVVVVV
    tabs.unshift(["Cycle Detected", () => <TreeCycle path={tree.cycle!} />]);
  }

  const tyCtx = {
    interner: internedTys,
    projections: tree.projectionValues
  };

  return (
    <TreeAppContext.TreeContext.Provider value={treeInfo}>
      <TyCtxt.Provider value={tyCtx}>
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
      </TyCtxt.Provider>
    </TreeAppContext.TreeContext.Provider>
  );
};

export default TreeApp;
