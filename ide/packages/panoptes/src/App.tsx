import {
  ErrorJumpTargetInfo,
  ExtensionToWebViewMsg,
  Filename,
  ObligationOutput,
} from "@argus/common/lib";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { useEffect, useState } from "react";

import Workspace from "./Workspace";
import { highlightedObligation } from "./signals";
import { bringToFront } from "./utilities/func";

function blingObserver(info: ErrorJumpTargetInfo) {
  console.debug(`Highlighting obligation ${info}`);
  highlightedObligation.set(info);
  return setTimeout(() => highlightedObligation.reset(), 1500);
}

const App = observer(
  ({
    data,
    target,
  }: {
    data: [Filename, ObligationOutput[]][];
    target?: ErrorJumpTargetInfo;
  }) => {
    const [openFiles, setOpenFiles] =
      useState<[Filename, ObligationOutput[]][]>(data);

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
          console.debug(
            "Current highlighted obligation",
            highlightedObligation
          );
          return blingObserver(payload);
        }

        case "open-file": {
          return setOpenFiles(currFiles => {
            const idx = _.findIndex(
              currFiles,
              ([filename, _]) => filename === payload.file
            );
            if (idx === -1) {
              return [[payload.file, payload.data], ...currFiles];
            }
            return bringToFront(currFiles, idx);
          });
        }

        case "reset": {
          // Re-render the open files.
          return setOpenFiles(payload.data);
        }

        // Everything else must be ignored.
        default:
          return;
      }
    };

    useEffect(() => {
      window.addEventListener("message", listener);
      if (target !== undefined) {
        blingObserver(target);
      }
      return () => window.removeEventListener("message", listener);
    }, []);

    const resetState = () => {
      return setOpenFiles(currFiles => currFiles);
    };

    return <Workspace files={openFiles} reset={resetState} />;
  }
);

export default App;
