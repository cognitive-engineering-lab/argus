import { ObligationOutput } from "@argus/common/lib";
import { Filename } from "@argus/common/lib";
import * as React from "react";
import * as ReactDOM from "react-dom/client";

import App from "./App";

declare global {
  interface Window {
    initialData: [Filename, ObligationOutput[]][];
  }
}

window.addEventListener("load", () => {
  console.log("Loading initialData", window.initialData);
  let root = ReactDOM.createRoot(document.getElementById("root")!);
  root.render(<App initialData={window.initialData} />);
});
