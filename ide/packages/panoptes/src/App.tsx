import { VSCodeButton } from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React, { useState } from "react";

import { ExtensionToWebViewMsg } from "@argus/common";
import ObligationApp from "./ObligationView/ObligationApp";
import TreeApp from "./TreeView/TreeApp";
import { vscode } from "./utilities/vscode";
import "./App.css";

const InternalApp = ({ message }: { message: ExtensionToWebViewMsg }) => {
  console.log("Rendering", message);

  const content =
    message.command === "none" ? (
      <span>Not loaded</span>
    ) : message.command === "obligations" ? (
      <ObligationApp obligations={_.flatten(message.obligations)} />
    ) : message.command === "tree" ? (
      <>
        <h2>Tree</h2>
        <TreeApp tree={message.tree} />
      </>
    ) : (
      <span>Unknown mode</span>
    );

  return <div>{content}</div>;
};

const App = () => {
  const [currentData, setCurrentData] = useState<ExtensionToWebViewMsg>({
    command: "none",
  });

  const listener = (e: MessageEvent) => {
    console.log("Received message from extension", e.data);

    setCurrentData(e.data);
  };

  React.useEffect(() => {
    window.addEventListener("message", listener);
    return () => window.removeEventListener("message", listener);
  }, []);

  const handleClick = () => {
    // Send a message back to the extension
    vscode.postMessage({ type: "FROM_WEBVIEW", command: "obligations" });
  };

  return (
    <div>
      <div>
        <VSCodeButton onClick={handleClick}>Fetch Obligations</VSCodeButton>
      </div>
      <InternalApp message={currentData} />
    </div>
  );
};

export default App;
