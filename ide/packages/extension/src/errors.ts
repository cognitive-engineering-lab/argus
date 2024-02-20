import { ArgusError } from "@argus/common/lib";
import cp from "child_process";
import newGithubIssueUrl from "new-github-issue-url";
import open from "open";
import os from "os";
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
        input: logs.join("\n"),
      });
    } catch (e: any) {
      log("Failed to call to paste.rs: ", e.toString());
    }

    const bts = "```";
    const logText = logUrl !== null ? `\n**Full log:** ${logUrl}` : ``;
    const url = newGithubIssueUrl({
      user: "gavinleroy",
      repo: "argus",
      body: `# Problem
<!-- Please describe the problem and how you encountered it. -->

# Logs
<!-- You don't need to add or change anything below this point. -->

**OS:** ${os.platform()} (${os.release()})
**VSCode:** ${vscode.version}
**Error message**
${bts}
${err}
${bts}
${logText}`,
    });

    open(url);
  }
};

export const showError = async (error: ArgusError) => {
  if (error.type === "build-error") {
    // TODO: is this how we want to show build errors?
    await showErrorDialog(error.error);
  } else if (error.type == "analysis-error") {
    await showErrorDialog(error.error);
  } else {
    await showErrorDialog("Unknown error");
  }
};

export async function lastError(context: vscode.ExtensionContext) {
  const error = context.workspaceState.get("err_log") as string;
  await showError({ type: "build-error", error });
}
