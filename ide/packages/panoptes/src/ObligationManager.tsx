import { Filename } from "@argus/common";
import {
  CharRange,
  Obligation,
  ObligationsInBody,
  SerializedTree,
} from "@argus/common/types";
import {
  VSCodeButton,
  VSCodeDivider,
  VSCodeProgressRing,
  VSCodeTextArea,
} from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React, { createContext, useContext, useEffect, useState } from "react";

import "./ObligationManager.css";
import TreeApp from "./TreeView/TreeApp";
// @ts-ignore
import { PrettyObligation } from "./Ty/print";
import { IcoChevronDown, IcoChevronUp } from "./utilities/icons";
import { testTree } from "./utilities/tree";
import { messageExtension } from "./utilities/vscode";

const FileContext = createContext<Filename | undefined>(undefined);

const ObligationTreeWrapper = ({
  range,
  obligation,
}: {
  range: CharRange;
  obligation: Obligation;
}) => {
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
          console.log("Received tree from extension", msg.tree);
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
      predicate: obligation,
      range: range,
    });

    // setTree(testTree);
    setIsTreeLoaded(true);
  }

  const content =
    tree === undefined ? (
      <>
        <p>Loading tree...</p>
        <VSCodeProgressRing />
      </>
    ) : (
      <TreeApp tree={tree} />
    );

  return <>{content}</>;
};

const ObligationCard = ({
  range,
  obligation,
}: {
  range: CharRange;
  obligation: Obligation;
}) => {
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
      onMouseEnter={addHighlight}
      onMouseLeave={removeHighlight}
    >
      <div className="PrettyObligationArea">
        <PrettyObligation obligation={obligation} />
      </div>
      <VSCodeButton
        className="ObligationButton"
        appearance="secondary"
        onClick={handleClick}
      >
        {isInfoVisible ? <IcoChevronUp /> : <IcoChevronDown />}
      </VSCodeButton>
      {isInfoVisible && (
        <ObligationTreeWrapper range={range} obligation={obligation} />
      )}
    </div>
  );
};

const ObligationBody = ({ osib }: { osib: ObligationsInBody }) => {
  const bodyRange = osib.range;
  const bodyName = osib.name;
  const [_successes, failures] = _.partition(
    osib.obligations,
    obligation => obligation.kind.type === "success"
  );

  // TODO: add code for the successes too
  return (
    <>
      <h3>Failed obligations in {bodyName}</h3>
      {_.map(failures, (obligation, idx) => {
        return (
          <ObligationCard range={bodyRange} obligation={obligation} key={idx} />
        );
      })}
    </>
  );
};

const ObligationManager = ({
  file,
  osibs,
}: {
  file: Filename;
  osibs: ObligationsInBody[];
}) => {
  return (
    <FileContext.Provider value={file}>
      {_.map(osibs, (osib, idx) => {
        return <ObligationBody osib={osib} key={idx} />;
      })}
    </FileContext.Provider>
  );
};

export default ObligationManager;
