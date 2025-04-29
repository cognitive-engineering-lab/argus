import type { MessageSystem } from "@argus/common/communication";
import type {
  PanoptesToSystemCmds,
  PanoptesToSystemMsg,
  SystemReturn
} from "@argus/common/lib";
import Panoptes, { UnsafeApp } from "@argus/panoptes";
import React from "react";
import ReactDOM from "react-dom/client";

import {
  ConfigConsts,
  type PanoptesConfig,
  maybeStringToConfig
} from "@argus/common/lib";
import { messageHandler } from "@estruyf/vscode/dist/client";

declare global {
  function initializeArgusBlocks(root: HTMLElement): void;
}

const system: MessageSystem = {
  postData<T extends PanoptesToSystemCmds>(
    command: T,
    body: Omit<PanoptesToSystemMsg<T>, "command">
  ) {
    return messageHandler.send(command, { command, ...body });
  },

  requestData<T extends PanoptesToSystemCmds>(
    command: T,
    body: Omit<PanoptesToSystemMsg<T>, "command">
  ): Promise<SystemReturn<T>> {
    return messageHandler.request<SystemReturn<T>>(command, {
      command,
      ...body
    });
  }
};

const Argus = ({ config }: { config: PanoptesConfig }) => {
  if (config.type !== "VSCODE_BACKING") return <Panoptes config={config} />;

  const spec = config.spec;
  config.evalMode = config.evalMode ?? "release";
  config.rankMode = config.rankMode ?? "inertia";
  const configNoUndef: Required<PanoptesConfig> = config as any;
  return <UnsafeApp config={configNoUndef} system={system} spec={spec} />;
};

window.initializeArgusBlocks = (root: HTMLElement) => {
  root
    .querySelectorAll<HTMLDivElement>(`.${ConfigConsts.EMBED_NAME}`)
    .forEach(elem => {
      elem.classList.remove(ConfigConsts.EMBED_NAME);
      elem.classList.add(ConfigConsts.PANOPTES_NAME);

      const panoConfig = maybeStringToConfig(elem.dataset.config);
      if (!panoConfig) throw new Error("missing data-config");

      const root = ReactDOM.createRoot(elem);
      root.render(<Argus config={panoConfig} />);
    });
};

window.addEventListener(
  "load",
  () => {
    console.info("Loading Panoptes WebView");
    window.initializeArgusBlocks(document.body);
  },
  false
);
