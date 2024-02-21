import {
  ExtensionToWebViewMsg,
  Filename,
  ObligationOutput,
} from "@argus/common/lib";
import { useSignals } from "@preact/signals-react/runtime";
import _ from "lodash";
import React, { useEffect, useState } from "react";

import Workspace from "./Workspace";
import { highlightedObligation } from "./signals";

const App = ({
  initialData,
}: {
  initialData: [Filename, ObligationOutput[]][];
}) => {
  useSignals();

  const [openFiles, setOpenFiles] =
    useState<[Filename, ObligationOutput[]][]>(initialData);

  // NOTE: this listener should only listen for posted messages, not
  // for things that could be an expected response from a webview request.
  const listener = (e: MessageEvent) => {
    const {
      command,
      payload,
    }: { command: string; payload: ExtensionToWebViewMsg } = e.data;

    console.debug("Received message from extension", command, payload);

    if (command != payload.command) {
      console.error("Received message with mismatched commands", e.data);
      return;
    }

    switch (payload.command) {
      case "open-error": {
        console.debug("Current highlighted obligation", highlightedObligation);
        highlightedObligation.value = payload;
        return setTimeout(() => (highlightedObligation.value = null), 1000);
      }

      case "open-file": {
        return setOpenFiles(currFiles => {
          if (_.find(currFiles, ([filename, _]) => filename === payload.file)) {
            return currFiles;
          }
          return [[payload.file, payload.data], ...currFiles];
        });
      }

      case "reset": {
        // Re-render the open files.
        return setOpenFiles(payload.data);
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
    return setOpenFiles(currFiles => currFiles);
  };

  return <Workspace files={openFiles} reset={resetState} />;
};

export default App;
