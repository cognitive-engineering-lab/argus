import { Obligation } from "@argus/common/types";
import {
  VSCodeButton,
  VSCodeDivider,
  VSCodeTextArea,
} from "@vscode/webview-ui-toolkit/react";
import classNames from "classnames";
import _ from "lodash";
import React from "react";

import { vscode } from "../utilities/vscode";
import "./ObligationApp.css";

const ObligationCard = ({ obligation }: { obligation: Obligation }) => {
  const className = classNames("Obligation", {
    success: obligation.type === "Success",
    failure: obligation.type === "Failure",
  });

  const addHighlight = () => {
    console.log("Highlighting range", obligation.range);

    vscode.postMessage({
      type: "FROM_WEBVIEW",
      command: "add-highlight",
      range: obligation.range,
    });
  };

  const removeHighlight = () => {
    console.log("Removing highlight", obligation.range);

    vscode.postMessage({
      type: "FROM_WEBVIEW",
      command: "remove-highlight",
      range: obligation.range,
    });
  };

  return (
    <div className="ObligationCard" onMouseEnter={addHighlight} onMouseLeave={removeHighlight}>
      <VSCodeButton className="ObligationButton" appearance="icon">
        <i className="codicon codicon-add" />
      </VSCodeButton>

      <VSCodeTextArea className={className} value={obligation.data} readOnly />
    </div>
  );
};

const ObligationApp = ({ obligations }: { obligations: Obligation[] }) => {
  const [successes, failures] = _.partition(
    obligations,
    obligation => obligation.type === "Success"
  );

  const doList = (obligations: Obligation[]) => {
    return (
      <>
        {_.map(obligations, (obligation, idx) => {
          return <ObligationCard obligation={obligation} key={idx} />;
        })}
      </>
    );
  };

  return (
    <>
      <h2>Failed obligations</h2>
      {doList(failures)}
      <VSCodeDivider />
      <h2>Successful obligations</h2>
      {doList(successes)}
    </>
  );
};

export default ObligationApp;
