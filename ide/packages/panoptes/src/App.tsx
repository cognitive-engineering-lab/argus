import { Filename } from "@argus/common";
import { ObligationsInBody } from "@argus/common/types";
import {
  VSCodeButton,
  VSCodePanelTab,
  VSCodePanelView,
  VSCodePanels,
  VSCodeProgressRing,
} from "@vscode/webview-ui-toolkit/react";
import _, { set } from "lodash";
import React, { useEffect, useState } from "react";
import { ErrorBoundary } from "react-error-boundary";

import "./App.css";
import ObligationManager from "./ObligationManager";
import { requestFromExtension } from "./utilities/vscode";

function basename(path: string) {
  return path.split("/").reverse()[0];
}

const WaitingOnObligations = () => {
  return (
    <>
      <p>Fetching obligations...</p>
      <VSCodeProgressRing />
    </>
  );
};

const OpenFile = ({ filename }: { filename: Filename }) => {
  const [obligations, setObligations] = useState<
    ObligationsInBody[] | undefined
  >(undefined);
  const [isLoading, setIsLoading] = useState(false);

  const handleClick = async () => {
    setIsLoading(true);
    setObligations(undefined);

    const obligations = await requestFromExtension<"obligations">({
      type: "FROM_WEBVIEW",
      command: "obligations",
      file: filename,
    });

    setObligations(obligations.obligations);
    setIsLoading(false);
  };

  return (
    <div>
      <div>
        <VSCodeButton onClick={handleClick}>Fetch Obligations</VSCodeButton>
      </div>
      {isLoading ? (
        <WaitingOnObligations />
      ) : (
        <ObligationManager file={filename} osibs={obligations!} />
      )}
    </div>
  );
};

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

const App = ({ initialFiles }: { initialFiles: Filename[] }) => {
  const [openFiles, setOpenFiles] = useState<[Filename, React.ReactElement][]>(
    _.map(initialFiles, filename => {
      return [filename, <OpenFile filename={filename} />];
    })
  );

  // NOTE: this listener should only listen for posted messages, not
  // for things that could be an expected response from a webview request.
  const listener = (e: MessageEvent) => {
    console.log("Received message from extension", e.data);
    const msg = e.data;

    // TODO: none of these messages are actually getting sent yet.
    switch (msg.command) {
      case "open-file": {
        setOpenFiles([
          ...openFiles,
          [msg.file, <OpenFile filename={msg.file} />],
        ]);
        return;
      }
      case "close-file": {
        setOpenFiles(
          _.filter(openFiles, ([filename, _]) => filename !== msg.filename)
        );
        return;
      }

      case "invalidate": {
        throw new Error("Invalidation, not yet implemented!");
      }

      // Everthing else must be ignored.
      default:
        return;
    }
  };

  useEffect(() => {
    window.addEventListener("message", listener);
    return () => window.removeEventListener("message", listener);
  }, []);

  const resetState = () => {
    setOpenFiles(
      _.map(openFiles, ([filename, _], i) => {
        return [filename, <OpenFile key={i} filename={filename} />];
      })
    );
  };

  return (
    <VSCodePanels>
      {_.map(openFiles, ([filename, _], idx) => {
        return (
          <VSCodePanelTab key={idx} id={`tab-${idx}`}>
            {basename(filename)}
          </VSCodePanelTab>
        );
      })}
      {_.map(openFiles, ([_, content], idx) => {
        return (
          <ErrorBoundary
            key={idx}
            FallbackComponent={FatalErrorPanel}
            onReset={resetState}
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

export default App;
