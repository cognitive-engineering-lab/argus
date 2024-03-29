import {
  BodyHash,
  ExprIdx,
  ObligationHash,
  ObligationsInBody,
  SerializedTree,
} from "@argus/common/bindings";
import { ErrorJumpTargetInfo } from "@argus/common/lib";
import { Filename } from "@argus/common/lib";
import * as React from "react";
import * as ReactDOM from "react-dom/client";

import App from "./App";
import {
  SystemPartialMap,
  createClosedMessageSystem,
  vscodeMessageSystem,
} from "./communication";

declare global {
  interface Window {
    data: [Filename, ObligationsInBody[]][];
    target?: ErrorJumpTargetInfo;
    createClosedSystem?: SystemPartialMap;
  }
}

window.addEventListener("load", () => {
  const root = ReactDOM.createRoot(document.getElementById("root")!);
  const messageSystem =
    window.createClosedSystem === undefined
      ? vscodeMessageSystem
      : createClosedMessageSystem(window.createClosedSystem);
  root.render(
    <App
      data={window.data}
      messageSystem={messageSystem}
      target={window.target}
    />
  );
});
