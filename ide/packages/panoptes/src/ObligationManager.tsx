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
import { postToExtension, requestFromExtension } from "./utilities/vscode";

const FileContext = createContext<Filename | undefined>(undefined);

const ObligationTreeWrapper = ({
  range,
  obligation,
}: {
  range: CharRange;
  obligation: Obligation;
}) => {
  const [isTreeLoaded, setIsTreeLoaded] = useState(false);
  const [tree, setTree] = useState<SerializedTree | undefined>(undefined);
  const file = useContext(FileContext)!;

  // FIXME: is this right, we only want o load things once.
  useEffect(() => {
    const getData = async () => {
      const tree = await requestFromExtension<"tree">({
        type: "FROM_WEBVIEW",
        file: file,
        command: "tree",
        predicate: obligation,
        range: range,
      });
      setTree(tree.tree);
      setIsTreeLoaded(true);
    };
    getData();
  }, []);

  const content =
    isTreeLoaded === false || tree === undefined ? (
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
    postToExtension({
      type: "FROM_WEBVIEW",
      file: file,
      command: "add-highlight",
      range: obligation.range,
    });
  };

  const removeHighlight = () => {
    console.log("Removing highlight", obligation.range);
    postToExtension({
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

  const doList = (kind: "Solved" | "Failed", obligations: Obligation[]) => {
    if (obligations.length === 0) {
      return;
    }

    const name = bodyName === undefined ? "" : "in " + bodyName;
    return (
      <>
        <h3>
          {kind} obligations {name}
        </h3>
        {_.map(obligations, (obligation, idx) => {
          return (
            <ObligationCard
              range={bodyRange}
              obligation={obligation}
              key={idx}
            />
          );
        })}
      </>
    );
  };

  // TODO: add code for the successes too
  return <div>{doList("Failed", failures)}</div>;
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
