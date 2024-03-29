import { ObligationsInBody } from "@argus/common/bindings";
import {
  ErrorJumpTargetInfo,
  Filename,
  SystemToPanoptesCmds,
  SystemToPanoptesMsg,
  isSysMsgOpenError,
  isSysMsgOpenFile,
  isSysMsgReset,
} from "@argus/common/lib";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { useEffect, useState } from "react";

import Workspace from "./Workspace";
import { MessageSystem, MessageSystemContext } from "./communication";
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
    messageSystem,
  }: {
    data: [Filename, ObligationsInBody[]][];
    messageSystem: MessageSystem;
    target?: ErrorJumpTargetInfo;
  }) => {
    const [openFiles, setOpenFiles] =
      useState<[Filename, ObligationsInBody[]][]>(data);

    // NOTE: this listener should only listen for posted messages, not
    // for things that could be an expected response from a webview request.
    const listener = (e: MessageEvent) => {
      const {
        payload,
      }: {
        payload: SystemToPanoptesMsg<SystemToPanoptesCmds>;
      } = e.data;

      console.debug("Received message from system", payload);

      if (isSysMsgOpenError(payload)) {
        return blingObserver(payload);
      } else if (isSysMsgOpenFile(payload)) {
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
      } else if (isSysMsgReset(payload)) {
        return setOpenFiles(payload.data);
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

    return (
      <MessageSystemContext.Provider value={messageSystem}>
        <Workspace files={openFiles} reset={resetState} />
      </MessageSystemContext.Provider>
    );
  }
);

export default App;
