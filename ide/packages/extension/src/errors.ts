import * as vscode from "vscode";

export type BuildError = {
  type: "BuildError";
  error: string;
};

export type ArgusError = {
  type: "AnalysisError";
  error: string;
};

export interface ArgusOutput<T> {
  type: "output";
  value: T;
}

export type ArgusResult<T> = ArgusOutput<T> | ArgusError | BuildError;

export let showErrorDialog = async (err: string) => {
  let outcome = await vscode.window.showErrorMessage(
    `Argus error: ${err}`,
    // 'Report bug',
    "Dismiss"
  );
};
