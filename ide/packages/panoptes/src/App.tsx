import {
  type MessageSystem,
  createClosedMessageSystem,
  vscodeMessageSystem
} from "@argus/common/communication";
import { AppContext } from "@argus/common/context";
import {
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
import { IcoComment } from "@argus/print/Icons";
import {
  DefPathRender,
  LocationActionable,
  type LocationActionableProps,
  ProjectionPathRender,
  TyCtxt
} from "@argus/print/context";
import { VSCodeCheckbox } from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { useEffect, useState } from "react";
import FillScreen, { Spacer } from "./FillScreen";

import "./App.css";
import type {
  DefPathRenderProps,
  ProjectPathRenderProps
} from "@argus/print/context";
import { PrintTyValue } from "@argus/print/lib";
import classNames from "classnames";
import MiniBuffer from "./MiniBuffer";
import Workspace from "./Workspace";
import { HighlightTargetStore, MiniBufferDataStore } from "./signals";

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
    return HighlightTargetStore.set(payload);
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

interface EventWithKeys {
  ctrlKey: boolean;
  metaKey: boolean;
}

/**
 * Check if the Ctrl or Meta key is pressed, used for jump to definition.
 */
const selectKeys = ({ ctrlKey, metaKey }: EventWithKeys) => ctrlKey || metaKey;

const mkLocationActionable =
  (system: MessageSystem) =>
  ({ children, location }: LocationActionableProps) => {
    const [hovered, setHovered] = useState(false);
    const [metaPressed, setMetaPressed] = useState(false);

    // FIXME: this doesn't seem like the best way to catch key presses for jump to definition.
    useEffect(() => {
      const keyDownListener = (ev: KeyboardEvent) =>
        setMetaPressed(selectKeys(ev));
      const keyUpListener = (ev: KeyboardEvent) =>
        setMetaPressed(selectKeys(ev));

      window.addEventListener("keydown", keyDownListener);
      window.addEventListener("keyup", keyUpListener);

      return () => {
        window.removeEventListener("keydown", keyDownListener);
        window.removeEventListener("keyup", keyUpListener);
      };
    }, []);

    // Hover actions for the entire path that allow jump to definition.
    const setHover = (ev: React.MouseEvent) => {
      // If the meta key was pressed outside of the window, we can catch it here as well.
      setMetaPressed(selectKeys(ev));
      setHovered(true);
    };
    const resetHover = () => setHovered(false);

    const click = (event: React.MouseEvent) => {
      if (selectKeys(event)) {
        event.preventDefault();
        event.stopPropagation();
        system.postData("jump-to-def", {
          type: "FROM_WEBVIEW",
          location
        });
      }
    };

    // Only allow the extra classes if there's not a location to jump to
    const cn = classNames("DefinitionWrapper", {
      hovered: location !== undefined && hovered,
      "meta-pressed": location !== undefined && metaPressed
    });

    return (
      // biome-ignore lint/a11y/useKeyWithClickEvents: TODO
      <span
        className={cn}
        onClick={click}
        onMouseEnter={setHover}
        onMouseLeave={resetHover}
      >
        {children}
      </span>
    );
  };

/**
 * Create a path renderer that puts full path definitions into the mini-buffer.
 */
const CustomPathRenderer = observer(
  ({ fullPath, ctx, Head, Rest }: DefPathRenderProps) => {
    // Hover actions for the Head symbol that show the full definition in the minibuffer.
    const setMB = () =>
      MiniBufferDataStore.set({ kind: "path", path: fullPath, ctx });
    const resetMB = () => MiniBufferDataStore.reset();
    // The click even and styling applying to the entire path, but the Symbol definition
    // in the MiniBuffer only applies to the Head segment.
    return (
      <>
        <span onMouseEnter={setMB} onMouseLeave={resetMB}>
          {Head}
        </span>
        {Rest}
      </>
    );
  }
);

const CustomProjectionRender = observer(
  ({ ctx, original, projection }: ProjectPathRenderProps) => {
    const setStore = () =>
      MiniBufferDataStore.set({
        kind: "projection",
        original,
        projection,
        ctx
      });
    const resetStore = () => MiniBufferDataStore.reset();
    return (
      <>
        <TyCtxt.Provider value={ctx}>
          <PrintTyValue ty={projection} />
        </TyCtxt.Provider>
        <span
          onMouseEnter={setStore}
          onMouseLeave={resetStore}
          style={{ verticalAlign: "super", fontSize: "0.25rem" }}
        >
          <IcoComment />
        </span>
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
    return () => window.removeEventListener("message", listen);
  }, []);

  useEffect(() => {
    if (config.target !== undefined) {
      HighlightTargetStore.set(config.target);
    }
  }, [config.target]);

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
      <Spacer />
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
              <ProjectionPathRender.Provider value={CustomProjectionRender}>
                <LocationActionable.Provider
                  value={mkLocationActionable(messageSystem)}
                >
                  <Workspace files={openFiles} reset={resetState} />
                  <FillScreen />
                </LocationActionable.Provider>
              </ProjectionPathRender.Provider>
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
