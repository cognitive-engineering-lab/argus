import vscode from "vscode";

import { CallArgus } from "@argus/common";
import { showErrorDialog } from "./errors";
import { log } from "./logging";
import { setup } from "./setup";
import { launchArgus } from "./viewloader";

export let globals: {
  backend: CallArgus;
};

// This method is called when your extension is activated
export async function activate(context: vscode.ExtensionContext) {
  log("Activating Argus ...");

  // TODO: this needs to be a little more robust.
  try {
    globals = {
      backend: () => {
        throw Error("Unreachable");
      },
    };

    let b = await setup(context);

    if (b == null) {
      showErrorDialog("Failed to setup Argus");
      return;
    }

    globals.backend = b;

    // Note, list must match commands listed in package.json.
    let commands: [string, () => Promise<void>][] = [
      ["launchArgus", async () => launchArgus(context.extensionUri)],
    ];

    commands.forEach(([name, func]) => {
      let disposable = vscode.commands.registerCommand(`argus.${name}`, func);
      context.subscriptions.push(disposable);
    });

  } catch (e: any) {
    showErrorDialog(e);
  }
}

// This method is called when your extension is deactivated
export function deactivate() {
  // TODO cleanup resources
}
