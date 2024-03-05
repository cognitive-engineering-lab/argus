import { ErrorJumpTargetInfo, ObligationOutput } from "@argus/common/lib";
import { Filename } from "@argus/common/lib";
import * as React from "react";
import * as ReactDOM from "react-dom/client";

import App from "./App";

declare global {
  interface Window {
    data: [Filename, ObligationOutput[]][];
    target?: ErrorJumpTargetInfo;
  }
}

window.addEventListener("load", () => {
  const root = ReactDOM.createRoot(document.getElementById("root")!);
  root.render(<App data={window.data} target={window.target} />);
});
