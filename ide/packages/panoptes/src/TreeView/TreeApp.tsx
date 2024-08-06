import TreeInfo from "@argus/common/TreeInfo";
import type { SerializedTree } from "@argus/common/bindings";
import { TreeAppContext } from "@argus/common/context";
import { TyCtxt } from "@argus/print/context";
import React from "react";

import BottomUp from "./BottomUp";
import Erotisi from "./Erotisi";
import Panels, { type PanelDescription, usePanelState } from "./Panels";
import TopDown from "./TopDown";
import "./TreeApp.css";

// FIXME: this shouldn't ever happen, if a properly hashed
// value is sent and returned. I need to think more about how to handle
// when we want to display "non-traditional" obligations.
const ErrorMessage = () => (
  <div className="Error">
    <p>Whoops! Something went wrong:</p>
    <pre>No debug information found.</pre>
  </div>
);

const TreeApp = ({
  tree,
  showHidden = false
}: {
  tree: SerializedTree | undefined;
  showHidden?: boolean;
}) => {
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

  const tyCtx = {
    interner: internedTys,
    projections: tree.projectionValues
  };

  // --------------------------------------
  // State dependent data for tab switching

  const [state, setState] = usePanelState();

  const tabs: PanelDescription[] = [
    {
      title: "Top Down",
      Content: () => <TopDown start={state?.node} />
    }
  ];

  if (treeInfo.failedSets().length > 0) {
    // Unshift to place this first
    // NOTE: the passing the TopDown panel ID is important, make sure it's always correct.
    // FIXME: we probably shouldn't hard-code that value here...
    tabs.unshift({
      title: "Bottom Up",
      Content: () => (
        <BottomUp
          jumpToTopDown={n =>
            // Callback passed to the BottomUp panel to jump to the TopDown panel.
            setState({ activePanel: 1, node: n, programatic: true })
          }
        />
      )
    });

    // Push to place this last
    tabs.push({ title: "Help Me", Content: Erotisi });
  }

  // HACK: we shouldn't test for eval mode here but Playwright is off on the button click.
  // if (tree.cycle !== undefined && evalMode === "release") {
  //   // FIXME: why do I need the '!' here? - - - - - --------  VVVVVVVV
  //   tabs.unshift({
  //     title: "Cycle Detected",
  //     Content: () => <TreeCycle path={tree.cycle!} />
  //   });
  // }

  return (
    <TreeAppContext.TreeContext.Provider value={treeInfo}>
      <TyCtxt.Provider value={tyCtx}>
        <div className="App">
          <Panels
            manager={[
              state.activePanel,
              n => setState({ activePanel: n }),
              state.programatic
            ]}
            description={tabs}
          />
        </div>
      </TyCtxt.Provider>
    </TreeAppContext.TreeContext.Provider>
  );
};

export default TreeApp;
