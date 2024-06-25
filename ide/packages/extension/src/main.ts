import vscode from "vscode";

import * as commands from "./commands";
import { CommandFactory, Ctx, fetchWorkspace } from "./ctx";
import { showErrorDialog } from "./errors";
import { log } from "./logging";

export let globals: {
  ctx: Ctx;
};

// This method is called when your extension is activated
export async function activate(context: vscode.ExtensionContext) {
  const wksp = fetchWorkspace();
  log("Activating Argus in workspace", wksp);
  if (wksp.kind !== "workspace-folder") {
    throw new Error("Argus only works in Rust workspaces");
  }

  // TODO: anything that needs to get registered on the window should be done here.
  // Initialize backend API for the workspace.
  const ctx = new Ctx(context, createCommands(), wksp);
  await ctx.setupBackend().catch(err => {
    showErrorDialog(`Cannot activate Argus extension: ${err.message}`);
    throw err;
  });

  log("Argus activated successfully");
  context.subscriptions.push(ctx);
  globals = {
    ctx,
  };
}

// This method is called when your extension is deactivated
export function deactivate() {
  globals.ctx.dispose();
  globals.ctx = undefined as any;
}

function createCommands(): Record<string, CommandFactory> {
  return {
    // Public commands that appear in the command palette and thus need to be listed in package.json.
    inspectWorkspace: { enabled: commands.inspect },
    cancelTasks: { enabled: commands.cancelTasks },

    // Private commands used internally, these should not appear in the command palette.
    openError: { enabled: commands.openError },
    lastError: { enabled: commands.lastError },
  };
}
