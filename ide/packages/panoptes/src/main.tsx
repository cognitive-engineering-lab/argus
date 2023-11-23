import { Filename } from "@argus/common";
import * as React from "react";
import * as ReactDOM from "react-dom";

import App from "./App";

declare global {
  interface Window {
    initialData: Filename[];
  }
}

window.addEventListener("load", () => {
  ReactDOM.render(
    <React.StrictMode>
      <App initialFiles={window.initialData} />
    </React.StrictMode>,
    document.getElementById("root") as HTMLElement
  );
});
