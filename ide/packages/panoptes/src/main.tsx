import * as React from "react";
import * as ReactDOM from "react-dom";
import { UnderlyingTree } from '@argus/common';
import { testy } from './testish';

import App from "./App";
import "./main.css";

declare global {
    interface Window {
      initialData: UnderlyingTree;
    }
  }

window.addEventListener("load", () => {
  // Does this need to be ReactDOM.hydrate?
  ReactDOM.render(
    // <App tree={window.initialData} />,
    <App tree={testy as UnderlyingTree} />,
    document.getElementById("root") as HTMLElement
  );
});