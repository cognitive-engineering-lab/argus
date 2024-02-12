import { VSCodeProgressRing } from "@vscode/webview-ui-toolkit/react";
import React from "react";

export const WaitingOn = ({ message }: { message: string | undefined }) => {
  const msg =
    message === undefined
      ? "Loading..."
      : `Loading ${message.toLowerCase()} ...`;
  return (
    <>
      <p>{msg}</p>
      <VSCodeProgressRing />
    </>
  );
};
