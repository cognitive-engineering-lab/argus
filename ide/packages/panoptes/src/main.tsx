import * as React from "react";
import * as ReactDOM from "react-dom";
import App from "./App";

window.addEventListener("load", () => {
  ReactDOM.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
    document.getElementById("root") as HTMLElement
  );
});
