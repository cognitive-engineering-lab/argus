import { getArgusIssueUrl } from "@argus/common/lib";
import { VSCodeProgressRing } from "@vscode/webview-ui-toolkit/react";
import React, { useContext, useEffect, useState } from "react";

import { AppContext } from "./utilities/context";

const PASTE_SUCCESS: number = 201;
const PASTE_PARTIAL: number = 206;

const ReportBugUrl = ({
  error,
  displayText = "Report Bug",
  logText,
}: {
  error: string;
  displayText?: string;
  logText?: string;
}) => {
  const initialLogState = logText ? undefined : `No available log :(`;
  const [logState, setLogState] = useState(initialLogState);
  const systemSpec = useContext(AppContext.SystemSpecContext)!;
  const errMsg = (e: string) => `Failed to call to paste.rs: '${e}'`;

  useEffect(() => {
    if (!logState) {
      fetch("https://paste.rs/", {
        method: "POST",
        mode: "cors",
        body: logText,
      })
        .then(response => response.json())
        .then(data => {
          if (data.status === PASTE_SUCCESS || data.status === PASTE_PARTIAL) {
            setLogState(data.body);
          } else {
            setLogState(errMsg(data.status));
          }
        })
        .catch(err => {
          setLogState(errMsg(err));
        });
    }
  }, []);

  return logState ? (
    <a
      href={getArgusIssueUrl(error, {
        logText: logState,
        ...systemSpec,
      })}
    >
      {displayText}
    </a>
  ) : (
    <>
      <VSCodeProgressRing /> building link ...
    </>
  );
};

export default ReportBugUrl;
