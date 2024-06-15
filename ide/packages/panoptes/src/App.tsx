import {
  createClosedMessageSystem,
  vscodeMessageSystem,
} from "@argus/common/communication";
import { AppContext } from "@argus/common/context";
import {
  ErrorJumpTargetInfo,
  EvaluationMode,
  FileInfo,
  PanoptesConfig,
  SystemSpec,
  isSysMsgHavoc,
  isSysMsgOpenError,
  isSysMsgOpenFile,
} from "@argus/common/lib";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { useEffect, useState } from "react";

import Workspace from "./Workspace";
import { highlightedObligation } from "./signals";

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

/**
 * Put all kinds of initial configuration state into a common format.
 */
function buildInitialData(config: PanoptesConfig): FileInfo[] {
  if (config.type === "VSCODE_BACKING") {
    return config.data;
  }

  const byName = _.groupBy(config.closedSystem, body => body.filename);
  return _.map(byName, (bodies, fn) => {
    return { fn, data: _.map(bodies, b => b.body) };
  });
}

const App = observer(({ config }: { config: PanoptesConfig }) => {
  const [openFiles, setOpenFiles] = useState(buildInitialData(config));
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
        const newEntry = {
          fn: payload.file,
          signature: payload.signature,
          data: payload.data,
        };
        const fileExists = _.find(currFiles, ({ fn }) => fn === payload.file);
        return fileExists ? currFiles : [...currFiles, newEntry];
      });
    } else if (isSysMsgHavoc(payload)) {
      return setOpenFiles([]);
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
