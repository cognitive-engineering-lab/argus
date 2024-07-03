import type { DefinedPath } from "@argus/common/bindings";
import {
  createClosedMessageSystem,
  vscodeMessageSystem
} from "@argus/common/communication";
import { AppContext } from "@argus/common/context";
import {
  type ErrorJumpTargetInfo,
  type EvaluationMode,
  type FileInfo,
  type PanoptesConfig,
  type SystemSpec,
  isSysMsgHavoc,
  isSysMsgOpenError,
  isSysMsgOpenFile,
  isSysMsgPin,
  isSysMsgUnpin
} from "@argus/common/lib";
import { DefPathRender } from "@argus/print/context";
import { VSCodeCheckbox } from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { useEffect, useState } from "react";

import "./App.css";
import type { TypeContext } from "@argus/print/context";
import MiniBuffer from "./MiniBuffer";
import Workspace from "./Workspace";
import { MiniBufferDataStore, highlightedObligation } from "./signals";

function blingObserver(info: ErrorJumpTargetInfo) {
  console.debug(`Highlighting obligation ${info}`);
  highlightedObligation.set(info);
  return setTimeout(() => highlightedObligation.reset(), 1500);
}

const webSysSpec: SystemSpec = {
  osPlatform: "web-bundle",
  osRelease: "web-bundle",
  vscodeVersion: "unknown"
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
  setOpenFiles: React.Dispatch<React.SetStateAction<FileInfo[]>>,
  pinMBData: () => void,
  unpinMBData: () => void
) {
  const {
    payload
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
        data: payload.data
      };
      const fileExists = _.find(currFiles, ({ fn }) => fn === payload.file);
      return fileExists ? currFiles : [...currFiles, newEntry];
    });
  } else if (isSysMsgHavoc(payload)) {
    return setOpenFiles([]);
  } else if (isSysMsgPin(payload)) {
    return pinMBData();
  } else if (isSysMsgUnpin(payload)) {
    return unpinMBData();
  }
}

/**
 * Path renderer that puts full path definitions into the mini-buffer.
 */
const CustomPathRenderer = observer(
  ({
    fullPath,
    ctx,
    Head,
    Rest
  }: {
    fullPath: DefinedPath;
    ctx: TypeContext;
    Head: React.ReactElement;
    Rest: React.ReactElement;
  }) => {
    const setStore = () =>
      MiniBufferDataStore.set({ kind: "path", path: fullPath, ctx });
    const resetStore = () => MiniBufferDataStore.reset();
    return (
      <>
        <span onMouseEnter={setStore} onMouseLeave={resetStore}>
          {Head}
        </span>
        {Rest}
      </>
    );
  }
);

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
    const listen = (e: MessageEvent) =>
      listener(
        e,
        setOpenFiles,
        () => MiniBufferDataStore.pin(),
        () => MiniBufferDataStore.unpin()
      );
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
  const WorkspaceContent = (
    <AppContext.ConfigurationContext.Provider value={configNoUndef}>
      <AppContext.SystemSpecContext.Provider value={systemSpec}>
        <AppContext.MessageSystemContext.Provider value={messageSystem}>
          <AppContext.ShowHiddenObligationsContext.Provider value={showHidden}>
            <DefPathRender.Provider value={CustomPathRenderer}>
              <Workspace files={openFiles} reset={resetState} />
            </DefPathRender.Provider>
          </AppContext.ShowHiddenObligationsContext.Provider>
        </AppContext.MessageSystemContext.Provider>
      </AppContext.SystemSpecContext.Provider>
    </AppContext.ConfigurationContext.Provider>
  );

  return (
    <div className="AppRoot">
      {Navbar}
      {WorkspaceContent}
      <MiniBuffer />
    </div>
  );
});

export default App;
