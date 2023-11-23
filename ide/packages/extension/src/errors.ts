import vscode from "vscode";

// TODO: lots more to do here.

export let showErrorDialog = async (err: string) => {
  let outcome = await vscode.window.showErrorMessage(
    `Argus error: ${err}`,
    // 'Report bug',
    "Dismiss"
  );
};
