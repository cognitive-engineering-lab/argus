import { Filename } from "@argus/common/lib";
import {
  VSCodePanelTab,
  VSCodePanelView,
  VSCodePanels,
} from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React from "react";
import { ErrorBoundary } from "react-error-boundary";

import "./Workspace.css";

// TODO: the workspace should manage a set of files. Currently the App is doing
// that, but the App should just launch the current workspace.

function basename(path: string) {
  return path.split("/").reverse()[0];
}

const FatalErrorPanel = ({ error, resetErrorBoundary }: any) => {
  return (
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
};

const Workspace = ({
  files,
  reset,
}: {
  files: [Filename, React.ReactElement][];
  reset: () => void;
}) => {
  return (
    <VSCodePanels>
      {_.map(files, ([filename, _], idx) => {
        return (
          <VSCodePanelTab key={idx} id={`tab-${idx}`}>
            {basename(filename)}
          </VSCodePanelTab>
        );
      })}
      {_.map(files, ([_, content], idx) => {
        return (
          <ErrorBoundary
            key={idx}
            FallbackComponent={FatalErrorPanel}
            onReset={reset}
          >
            <VSCodePanelView key={idx} id={`view-${idx}`}>
              {content}
            </VSCodePanelView>
          </ErrorBoundary>
        );
      })}
    </VSCodePanels>
  );
};

export default Workspace;
