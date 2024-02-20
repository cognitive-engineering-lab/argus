import { ObligationsInBody } from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";
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
  <div>
    <p>
      Whoops! This is not a drill, a fatal error occurred. Please{" "}
      <a href="https://github.com/gavinleroy/argus/issues/new">
        report this error
      </a>{" "}
      to the Argus team, and include the following information:
    </p>
    <pre>{error.message}</pre>
    <button onClick={resetErrorBoundary}>Reset Argus</button>
  </div>
);

const Workspace = ({
  files,
  reset,
}: {
  files: [Filename, ObligationsInBody[]][];
  reset: () => void;
}) => {
  const [showHidden, setShowHidden] = useState(false);
  const toggleHidden = () => setShowHidden(!showHidden);

  const checkbox = (
    <div style={{ position: "fixed", top: "0", right: "0" }}>
      <VSCodeCheckbox onChange={toggleHidden} checked={showHidden}>
        Show hidden information
      </VSCodeCheckbox>
    </div>
  );

  const tabs = _.map(files, ([filename, _], idx) => (
    <VSCodePanelTab key={idx} id={`tab-${idx}`}>
      {basename(filename)}
    </VSCodePanelTab>
  ));

  const fileComponents = _.map(files, ([filename, content], idx) => (
    <ErrorBoundary
      key={idx}
      FallbackComponent={FatalErrorPanel}
      onReset={reset}
    >
      <VSCodePanelView
        key={idx}
        id={`view-${idx}`}
        style={{ paddingLeft: 0, paddingRight: 0 }}
      >
        <File file={filename} osibs={content} showHidden={showHidden} />
      </VSCodePanelView>
    </ErrorBoundary>
  ));

  return (
    <>
      {checkbox}
      <VSCodePanels>
        {tabs}
        {fileComponents}
      </VSCodePanels>
    </>
  );
};

export default Workspace;
