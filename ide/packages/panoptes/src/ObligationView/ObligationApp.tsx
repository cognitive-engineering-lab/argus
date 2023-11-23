import { Filename } from "@argus/common";
import { Obligation, SerializedTree } from "@argus/common/types";
import {
  VSCodeButton,
  VSCodeDivider,
  VSCodeProgressRing,
  VSCodeTextArea,
} from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React, { createContext, useContext, useEffect, useState } from "react";

import TreeApp from "../TreeView/TreeApp";
import { messageExtension } from "../utilities/vscode";
import "./ObligationApp.css";

const FileContext = createContext<Filename | undefined>(undefined);

const ObligationProofTreeWrapper = () => {
  const [isTreeLoaded, setIsTreeLoaded] = useState(false);
  const [tree, setTree] = useState<SerializedTree[] | undefined>(undefined);
  const file = useContext(FileContext)!;

  const listener = (e: MessageEvent) => {
    console.log("Received message from extension", e.data);

    const msg = e.data;
    if (msg.type !== "FROM_EXTENSION") {
      // FIXME: yeah, don't throw an error. Just ignore it.
      throw new Error(`Unexpected message type ${msg}`);
    }

    switch (msg.command) {
      case "tree": {
        if (tree === undefined) {
          setTree(msg.tree);
        }
        return;
      }
      default: {
        // Ignore all other cases.
        return;
      }
    }
  };

  useEffect(() => {
    window.addEventListener("message", listener);
    return () => window.removeEventListener("message", listener);
  }, []);

  // Load the tree once;
  // FIXME: this isn't going to work, come back and fix.
  if (!isTreeLoaded) {
    messageExtension({
      type: "FROM_WEBVIEW",
      file: file,
      command: "tree",
      line: 0,
      column: 0,
    });
    setIsTreeLoaded(true);
  }

  const content =
    tree === undefined ? (
      <>
        <VSCodeProgressRing />
        <p>Loading tree...</p>
      </>
    ) : (
      <TreeApp tree={tree} />
    );

  return <>{content}</>;
};

const ObligationCard = ({ obligation }: { obligation: Obligation }) => {
  const [isInfoVisible, setIsInfoVisible] = useState(false);
  const file = useContext(FileContext)!;

  const addHighlight = () => {
    console.log("Highlighting range", obligation.range);

    messageExtension({
      type: "FROM_WEBVIEW",
      file: file,
      command: "add-highlight",
      range: obligation.range,
    });
  };

  const removeHighlight = () => {
    console.log("Removing highlight", obligation.range);

    messageExtension({
      type: "FROM_WEBVIEW",
      file: file,
      command: "remove-highlight",
      range: obligation.range,
    });
  };

  const handleClick = () => {
    setIsInfoVisible(!isInfoVisible);
  };

  return (
    <div
      className="ObligationCard"
      onClick={handleClick}
      onMouseEnter={addHighlight}
      onMouseLeave={removeHighlight}
    >
      <VSCodeButton className="ObligationButton" appearance="icon">
        <i className="codicon codicon-add" />
      </VSCodeButton>
      <VSCodeTextArea value={obligation.data} readOnly />
      {isInfoVisible && <ObligationProofTreeWrapper />}
    </div>
  );
};

const ObligationApp = ({
  file,
  obligations,
}: {
  file: Filename;
  obligations: Obligation[] | undefined;
}) => {
  if (obligations === undefined) {
    return <p>Obligations not loaded</p>;
  }

  const [successes, failures] = _.partition(
    obligations,
    obligation => obligation.type === "Success"
  );

  const doList = (obligations: Obligation[]) => {
    const os = _.sortBy(obligations, obligation => obligation.range.start.line);
    const uqs = _.uniqBy(os, obligation => obligation.data);
    return (
      <>
        {_.map(uqs, (obligation, idx) => {
          return <ObligationCard obligation={obligation} key={idx} />;
        })}
      </>
    );
  };

  return (
    <FileContext.Provider value={file}>
      <h2>Failed obligations</h2>
      {doList(failures)}
      <VSCodeDivider />
      <h2>Successful obligations</h2>
      {doList(successes)}
    </FileContext.Provider>
  );
};

export default ObligationApp;
