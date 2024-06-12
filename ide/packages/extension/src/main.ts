import vscode from "vscode";

import * as commands from "./commands";
import { CommandFactory, Ctx, fetchWorkspace } from "./ctx";
import { showErrorDialog } from "./errors";
import { log } from "./logging";
import { StatusBar } from "./statusbar";

export let globals: {
  ctx: Ctx;
  statusBar: StatusBar;
};

// This method is called when your extension is activated
export async function activate(context: vscode.ExtensionContext) {
  globals = {
    ctx: undefined as any,
    statusBar: new StatusBar(context),
  };

  log("Activating Argus ...");

  const ctx = new Ctx(context, createCommands(), fetchWorkspace());
  const api = await activateBackend(ctx).catch(err => {
    showErrorDialog(`Cannot activate Argus extension: ${err.message}`);
    throw err;
  });

  globals = {
    ...globals,
    ctx: api,
  };
}

// This method is called when your extension is deactivated
export function deactivate() {
  // TODO: cleanup resources
}

async function activateBackend(ctx: Ctx): Promise<Ctx> {
  // TODO: anything that needs to get registered on the window should be done here.

  // Initialize backend API for the workspace.
  await ctx.setupBackend();

  return ctx;
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
