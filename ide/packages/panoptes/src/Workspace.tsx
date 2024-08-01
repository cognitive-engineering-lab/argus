import type { FileInfo } from "@argus/common/lib";
import ReportBugUrl from "@argus/print/ReportBugUrl";
import _ from "lodash";
import React from "react";
import { ErrorBoundary } from "react-error-boundary";

import File from "./File";
import Panels, { type PanelDescription } from "./TreeView/Panels";

function basename(path: string) {
  return path.split("/").reverse()[0];
}

const FatalErrorPanel = ({ error, resetErrorBoundary }: any) => (
  <div className="ErrorPanel">
    Whoops! This is not a drill, a fatal error occurred. Please{" "}
    <ReportBugUrl displayText="click here" error={error.message} />
    to report this error to the Argus team.
    <button type="button" onClick={resetErrorBoundary}>
      Reset Argus
    </button>
  </div>
);

const Workspace = ({
  files,
  reset
}: {
  files: FileInfo[];
  reset: () => void;
}) => {
  const viewProps = {
    style: {
      paddingLeft: 0,
      paddingRight: 0
    }
  };
  const tabs: PanelDescription[] = _.map(files, ({ fn, data }, idx) => {
    return {
      viewProps,
      title: basename(fn),
      Content: (
        <ErrorBoundary
          key={idx}
          FallbackComponent={FatalErrorPanel}
          onReset={reset}
        >
          <File file={fn} osibs={data} />
        </ErrorBoundary>
      )
    };
  });

  return (
    <div className="workspace-area">
      <Panels description={tabs} />
    </div>
  );
};

export default Workspace;
