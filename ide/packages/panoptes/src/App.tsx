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
import { VSCodeCheckbox } from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { useEffect, useState } from "react";

import "./App.css";
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

// NOTE: this listener should only listen for posted messages, not
// for things that could be an expected response from a webview request.
function listener(
  e: MessageEvent,
  setOpenFiles: React.Dispatch<React.SetStateAction<FileInfo[]>>
) {
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
}

const App = observer(({ config }: { config: PanoptesConfig }) => {
  const [openFiles, setOpenFiles] = useState(buildInitialData(config));
  const [showHidden, setShowHidden] = useState(false);

  const messageSystem =
    config.type === "WEB_BUNDLE"
      ? createClosedMessageSystem(config.closedSystem)
      : vscodeMessageSystem;
  const systemSpec =
    config.type === "VSCODE_BACKING" ? config.spec : webSysSpec;

  config.evalMode = config.evalMode ?? "release";
  const configNoUndef: PanoptesConfig & { evalMode: EvaluationMode } =
    config as any;

  useEffect(() => {
    const listen = (e: MessageEvent) => listener(e, setOpenFiles);
    window.addEventListener("message", listen);
    if (config.target !== undefined) {
      blingObserver(config.target);
    }
    return () => window.removeEventListener("message", listen);
  }, []);

  const Navbar = (
    <>
      <div className="app-nav">
        <VSCodeCheckbox
          onChange={() => setShowHidden(!showHidden)}
          checked={showHidden}
        >
          Show hidden information
        </VSCodeCheckbox>
      </div>
      <div className="spacer">{"\u00A0"}</div>
    </>
  );

  // Rerender the App without changing the base files.
  const resetState = () => setOpenFiles(currFiles => currFiles);
  return (
    <div className="AppRoot">
      {Navbar}
      <AppContext.ConfigurationContext.Provider value={configNoUndef}>
        <AppContext.SystemSpecContext.Provider value={systemSpec}>
          <AppContext.MessageSystemContext.Provider value={messageSystem}>
            <AppContext.ShowHiddenObligationsContext.Provider
              value={showHidden}
            >
              <Workspace files={openFiles} reset={resetState} />
            </AppContext.ShowHiddenObligationsContext.Provider>
          </AppContext.MessageSystemContext.Provider>
        </AppContext.SystemSpecContext.Provider>
      </AppContext.ConfigurationContext.Provider>
    </div>
  );
});

export default App;
