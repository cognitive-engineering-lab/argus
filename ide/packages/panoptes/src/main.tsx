import { ConfigConsts, maybeStringToConfig } from "@argus/common/lib";
import * as React from "react";
import * as ReactDOM from "react-dom/client";

import App from "./App";

declare global {
  function initializeArgusBlocks(root: HTMLElement): void;
}

window.initializeArgusBlocks = (root: HTMLElement) => {
  root
    .querySelectorAll<HTMLDivElement>("." + ConfigConsts.EMBED_NAME)
    .forEach(elem => {
      elem.classList.remove(ConfigConsts.EMBED_NAME);
      elem.classList.add(ConfigConsts.PANOPTES_NAME);

      const panoConfig = maybeStringToConfig(elem.dataset.config);
      if (!panoConfig) throw new Error(`missing data-config`);

      const root = ReactDOM.createRoot(elem);
      root.render(<App config={panoConfig} />);
    });
};

window.addEventListener(
  "load",
  () => window.initializeArgusBlocks(document.body),
  false
);
