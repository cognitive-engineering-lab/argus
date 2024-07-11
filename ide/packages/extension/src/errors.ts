import cp from "node:child_process";
import os from "node:os";
import { type ArgusError, getArgusIssueUrl } from "@argus/common/lib";
import open from "open";
import vscode from "vscode";

import { log, logs } from "./logging";

export const showErrorDialog = async (err: string) => {
  const outcome = await vscode.window.showErrorMessage(
    `Argus error: ${err}`,
    "Report bug",
    "Dismiss"
  );

  if (outcome === "Report bug") {
    let logUrl = null;
    try {
      logUrl = cp.execSync("curl --data-binary @- https://paste.rs/", {
        input: logs.join("\n")
      });
    } catch (e: any) {
      log("Failed to call to paste.rs: ", e.toString());
    }

    const url = getArgusIssueUrl(err, {
      osPlatform: os.platform(),
      osRelease: os.release(),
      vscodeVersion: vscode.version,
      logText: logUrl !== null ? `\n**Full log:** ${logUrl}` : ""
    });

    open(url);
  }
};

export const ARGUS_ERR_LOG_KEY = "argus_err_log";

export const showError = async (error: ArgusError) => {
  if (error.type === "build-error") {
    // TODO: is this how we want to show build errors?
    await showErrorDialog(error.error);
  } else if (error.type === "analysis-error") {
    await showErrorDialog(error.error);
  } else {
    await showErrorDialog("Unknown error");
  }
};

export async function lastError(context: vscode.ExtensionContext) {
  const error = context.workspaceState.get(ARGUS_ERR_LOG_KEY) as string;
  await showError({ type: "build-error", error });
}
