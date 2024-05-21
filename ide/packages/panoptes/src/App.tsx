import {
  ErrorJumpTargetInfo,
  EvaluationMode,
  PanoptesConfig,
  SystemSpec,
  isSysMsgOpenError,
  isSysMsgOpenFile,
  isSysMsgReset,
} from "@argus/common/lib";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { useEffect, useState } from "react";

import Workspace from "./Workspace";
import {
  createClosedMessageSystem,
  vscodeMessageSystem,
} from "./communication";
import { highlightedObligation } from "./signals";
import { AppContext } from "./utilities/context";
import { bringToFront } from "./utilities/func";

function blingObserver(info: ErrorJumpTargetInfo) {
  console.debug(`Highlighting obligation ${info}`);
  highlightedObligation.set(info);
  return setTimeout(() => highlightedObligation.reset(), 1500);
}

const webSysSpec: SystemSpec = {
  osPlatform: "web-bundle",
  osRelease: "web-bundle",
  vscodeVersion: "unknown",
};

const App = observer(({ config }: { config: PanoptesConfig }) => {
  const [openFiles, setOpenFiles] = useState(config.data);
  const messageSystem =
    config.type === "WEB_BUNDLE"
      ? createClosedMessageSystem(config.closedSystem)
      : vscodeMessageSystem;
  const systemSpec =
    config.type === "VSCODE_BACKING" ? config.spec : webSysSpec;

  config.evalMode = config.evalMode ?? "release";
  const configNoUndef: PanoptesConfig & { evalMode: EvaluationMode } =
    config as any;

  // NOTE: this listener should only listen for posted messages, not
  // for things that could be an expected response from a webview request.
  const listener = (e: MessageEvent) => {
    const {
      payload,
    }: {
      payload: any;
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
    if (config.target !== undefined) {
      blingObserver(config.target);
    }
    return () => window.removeEventListener("message", listener);
  }, []);

  const resetState = () => {
    return setOpenFiles(currFiles => currFiles);
  };

  return (
    <AppContext.ConfigurationContext.Provider value={configNoUndef}>
      <AppContext.SystemSpecContext.Provider value={systemSpec}>
        <AppContext.MessageSystemContext.Provider value={messageSystem}>
          <Workspace files={openFiles} reset={resetState} />
        </AppContext.MessageSystemContext.Provider>
      </AppContext.SystemSpecContext.Provider>
    </AppContext.ConfigurationContext.Provider>
  );
});

export default App;
