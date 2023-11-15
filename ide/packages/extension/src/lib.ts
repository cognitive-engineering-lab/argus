// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from "vscode";

import { showErrorDialog } from "./errors";
import { log } from "./logging";
import { CallArgus, setup } from "./setup";
import { displayAll } from "./show";

export let globals: {
  backend: CallArgus;
};

// This method is called when your extension is activated
// Your extension is activated the very first time the command is executed
export async function activate(context: vscode.ExtensionContext) {
  log("Activating Argus ...");

  try {
    globals = {
      backend: () => {
        throw Error("Unreachable");
      },
    };

    let b = await setup(context);
    if (b == null) {
      return;
    }
    globals.backend = b;

    // Note, list must match commands listed in package.json.
    let commands: [string, () => Promise<void>][] = [
      ["showAllTrees", async () => displayAll(context.extensionUri)],
    ];

    commands.forEach(([name, func]) => {
      let disposable = vscode.commands.registerCommand(`argus.${name}`, func);
      context.subscriptions.push(disposable);
    });
  } catch (e: any) {
    showErrorDialog(e);
  }

  log("Activating Argus ...");
}

// This method is called when your extension is deactivated
export function deactivate() {}
