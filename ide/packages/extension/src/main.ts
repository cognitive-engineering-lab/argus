import type vscode from "vscode";

import * as commands from "./commands";
import { type CommandFactory, Ctx, fetchWorkspace } from "./ctx";
import { CtxInit } from "./ctx";
import { log } from "./logging";
import { setup } from "./setup";

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
  const initialCtxt = new CtxInit(context, wksp);
  log("setting up Argus backend");
  const backend = await setup(initialCtxt);
  if (backend == null) {
    return;
  }

  const ctx = await Ctx.make(initialCtxt, backend, createCommands());
  log("Argus activated successfully");
  context.subscriptions.push(ctx);
  globals = {
    ctx
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
    pinMBData: { enabled: commands.pinMBData },
    unpinMBData: { enabled: commands.unpinMBData },

    // Private commands used internally, these should not appear in the command palette.
    openError: { enabled: commands.openError },
    lastError: { enabled: commands.lastError }
  };
}
