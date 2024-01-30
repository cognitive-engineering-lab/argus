import { ObligationOutput } from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";
import * as React from "react";
import * as ReactDOM from "react-dom";

import App from "./App";

declare global {
  interface Window {
    initialData: [Filename, ObligationOutput[]][];
  }
}

window.addEventListener("load", () => {
  console.log("Loading initialData", window.initialData);
  ReactDOM.render(
    <React.StrictMode>
      <App initialData={window.initialData} />
    </React.StrictMode>,
    document.getElementById("root") as HTMLElement
  );
});
