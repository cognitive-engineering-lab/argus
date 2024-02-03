import {
  ExtensionToWebViewMsg,
  Filename,
  ObligationOutput,
} from "@argus/common/lib";
import _ from "lodash";
import React, { RefObject, useEffect, useState } from "react";

import { errorCardId, obligationCardId } from "./File";
import Workspace from "./Workspace";

// FIXME: this is wrong, expanding the nodes with JS doesn't cause
// a re-render in React. Better to have a signal that collapsible
// elements can listen to.
function highlightIntoView(id: string) {
  const elem = document.getElementById(id);
  const className = "bling";
  if (elem !== null) {
    // Expand each parent collapsible element.
    var a = elem.parentElement;
    while (a) {
      if (a.id === "Collapsible") {
        a.classList.remove("collapsed");
      }
      a = a.parentElement;
    }

    elem.scrollIntoView();
    elem.classList.add(className);
    setTimeout(() => elem.classList.remove(className), 1000); // Let the emphasis stay for 1 second.
  } else {
    console.error(`Couldn't find element with id ${id} to highlight`);
  }
}

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
      case "open-error": {
        highlightIntoView(
          errorCardId(
            payload.file,
            payload.bodyIdx,
            payload.errIdx,
            payload.errType
          )
        );
        return;
      }

      case "bling": {
        highlightIntoView(obligationCardId(payload.file, payload.oblHash));
        return;
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
