import type { FileInfo } from "@argus/common/lib";
import ReportBugUrl from "@argus/print/ReportBugUrl";
import _ from "lodash";
import React, { useEffect } from "react";
import { ErrorBoundary } from "react-error-boundary";

import { observer } from "mobx-react";
import File from "./File";
import Panels, {
  type PanelDescription,
  usePanelState
} from "./TreeView/Panels";
import { HighlightTargetStore } from "./signals";

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

const Workspace = observer(
  ({
    files,
    reset
  }: {
    files: FileInfo[];
    reset: () => void;
  }) => {
    const [state, setState] = usePanelState();

    useEffect(() => {
      if (HighlightTargetStore.value?.file === undefined) return;

      const idx = _.findIndex(
        files,
        ({ fn }) => fn === HighlightTargetStore.value?.file
      );
      if (0 <= idx && idx !== state.activePanel)
        setState({ activePanel: idx, programatic: true });
    }, [HighlightTargetStore.value?.file]);

    const viewProps = {
      style: {
        paddingLeft: 0,
        paddingRight: 0
      }
    };

    const tabs: PanelDescription[] = _.map(files, ({ fn, data }, idx) => {
      return {
        fn,
        viewProps,
        title: basename(fn),
        Content: () => (
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
        <Panels
          description={tabs}
          manager={[
            state.activePanel,
            n => setState({ activePanel: n }),
            state.programatic
          ]}
        />
      </div>
    );
  }
);

export default Workspace;
