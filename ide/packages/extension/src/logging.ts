import * as vscode from "vscode";

export const asyncWithProgress = async <T>(thunk: () => Promise<T>) => {
  return vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "Running",
      cancellable: true,
    },
    async progress => {
      progress.report({ increment: 0 });
      let v = await thunk();
      progress.report({
        increment: 100,
        message: "Done",
      });
      return v;
    }
  );
};

let channel = vscode.window.createOutputChannel("Argus");
export let logs: string[] = [];
export let log = (...strs: any[]) => {
  let s = strs.map(obj => String(obj)).join("\t");
  logs.push(s);
  channel.appendLine(s);
  console.debug(...strs);
};
