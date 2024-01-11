import { ObligationHash } from "@argus/common/bindings";
import { CallArgus, Filename } from "@argus/common/lib";
import vscode from "vscode";

import { showErrorDialog } from "./errors";
import { log } from "./logging";
import { setup } from "./setup";
import {
  blingObligation,
  launchArgus,
  onChange as webviewOnChange,
} from "./view";

// FIXME: HACK: VSCode commands cannot have arguments, nor can Markdown
// links call arbitrary code. This is a workaround but there is
// certainly a better way to do this.
export type CommandParameters = { obligation: ObligationHash; file: Filename };

export let globals: {
  backend: CallArgus;
  params?: CommandParameters;
  diagnosticCollection: vscode.DiagnosticCollection;
};

// This method is called when your extension is activated
export async function activate(context: vscode.ExtensionContext) {
  log("Activating Argus ...");

  try {
    globals = {
      backend: () => {
        throw Error("Unreachable");
      },
      diagnosticCollection:
        vscode.languages.createDiagnosticCollection("argus"),
    };

    context.subscriptions.push(globals.diagnosticCollection);

    let b = await setup(context);

    if (b == null) {
      showErrorDialog("Failed to setup Argus");
      return;
    }

    // Compile the workspace with the Argus version of rustc.
    await b(["preload"], true);

    globals.backend = b;

    // Note, list must match commands listed in package.json.
    let commands: [string, () => Promise<void>][] = [
      ["launchArgus", async () => launchArgus(context.extensionUri)],
      ["blingObligation", async () => blingObligation(context.extensionUri)],
    ];

    commands.forEach(([name, func]) => {
      let disposable = vscode.commands.registerCommand(`argus.${name}`, func);
      context.subscriptions.push(disposable);
    });
  } catch (e: any) {
    showErrorDialog(e);
  }
}

export function onChange() {
  webviewOnChange();
}

// This method is called when your extension is deactivated
export function deactivate() {
  // TODO cleanup resources
}
