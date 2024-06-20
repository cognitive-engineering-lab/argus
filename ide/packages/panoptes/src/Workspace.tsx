import { AppContext } from "@argus/common/context";
import { FileInfo } from "@argus/common/lib";
import ReportBugUrl from "@argus/print/ReportBugUrl";
import {
  VSCodeCheckbox,
  VSCodePanelTab,
  VSCodePanelView,
  VSCodePanels,
} from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React, { useState } from "react";
import { ErrorBoundary } from "react-error-boundary";

import File from "./File";
import "./Workspace.css";

// TODO: the workspace should manage a set of files. Currently the App is doing
// that, but the App should just launch the current workspace.

function basename(path: string) {
  return path.split("/").reverse()[0];
}

const FatalErrorPanel = ({ error, resetErrorBoundary }: any) => (
  <div className="ErrorPanel">
    Whoops! This is not a drill, a fatal error occurred. Please{" "}
    <ReportBugUrl displayText="click here" error={error.message} />
    to report this error to the Argus team.
    <button onClick={resetErrorBoundary}>Reset Argus</button>
  </div>
);

const Workspace = ({
  files,
  reset,
}: {
  files: FileInfo[];
  reset: () => void;
}) => {
  const [active, setActive] = useState(0);
  const [showHidden, setShowHidden] = useState(false);
  const toggleHidden = () => setShowHidden(!showHidden);

  const Navbar = () => (
    <>
      <div className="workspace-nav">
        <VSCodeCheckbox onChange={toggleHidden} checked={showHidden}>
          Show hidden information
        </VSCodeCheckbox>
      </div>
      <div className="spacer">{"\u00A0"}</div>
    </>
  );

  const mkActiveSet = (idx: number) => () => setActive(idx);
  const tabName = (idx: number) => `tab-${idx}`;

  const tabs = _.map(files, ({ fn }, idx) => (
    <VSCodePanelTab key={idx} id={tabName(idx)} onClick={mkActiveSet(idx)}>
      {basename(fn)}
    </VSCodePanelTab>
  ));

  const fileComponents = _.map(files, ({ fn, data }, idx) => (
    <VSCodePanelView key={idx} style={{ paddingLeft: 0, paddingRight: 0 }}>
      <ErrorBoundary
        key={idx}
        FallbackComponent={FatalErrorPanel}
        onReset={reset}
      >
        <File file={fn} osibs={data} />
      </ErrorBoundary>
    </VSCodePanelView>
  ));

  return (
    <AppContext.ShowHiddenObligationsContext.Provider value={showHidden}>
      <Navbar />
      <div className="workspace-area">
        <VSCodePanels activeid={tabName(active)}>
          {tabs}
          {fileComponents}
        </VSCodePanels>
      </div>
    </AppContext.ShowHiddenObligationsContext.Provider>
  );
};

export default Workspace;
