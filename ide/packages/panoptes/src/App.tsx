import {
  ExtensionToWebViewMsg,
  Filename,
  ObligationOutput,
} from "@argus/common/lib";
import _ from "lodash";
import React, { useEffect, useState } from "react";

import Workspace from "./Workspace";
import { highlightedObligation } from "./signals";

// // FIXME: this is wrong, expanding the nodes with JS doesn't cause
// // a re-render in React. Better to have a signal that collapsible
// // elements can listen to.
// function highlightIntoView(id: string) {
//   const elem = document.getElementById(id);
//   const className = "bling";
//     elem.scrollIntoView();
//     elem.classList.add(className);
//   } else {
//     console.error(`Couldn't find element with id ${id} to highlight`);
//   }
// }

const App = ({
  initialData,
}: {
  initialData: [Filename, ObligationOutput[]][];
}) => {
  const [openFiles, setOpenFiles] =
    useState<[Filename, ObligationOutput[]][]>(initialData);

  // NOTE: this listener should only listen for posted messages, not
  // for things that could be an expected response from a webview request.
  const listener = (e: MessageEvent) => {
    console.log("Received message from extension", e.data);
    const {
      command,
      payload,
    }: { command: string; payload: ExtensionToWebViewMsg } = e.data;

    if (command != payload.command) {
      console.log("Received message with mismatched commands", e.data);
      return;
    }

    switch (payload.command) {
      case "highlight": {
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
