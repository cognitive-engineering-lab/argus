import { ArgusArgs, ArgusResult, Filename } from "@argus/common/lib";
import vscode from "vscode";

import * as commands from "./commands";
import { CommandFactory, Ctx, fetchWorkspace } from "./ctx";
import { log } from "./logging";

export interface ArgusCtx {
  backend<T>(_args: ArgusArgs, _no_output?: boolean): Promise<ArgusResult<T>>;
}

export let globals: {
  ctx: Ctx;
};

// This method is called when your extension is activated
export async function activate(context: vscode.ExtensionContext) {
  log("Activating Argus ...");
  const ctx = new Ctx(context, createCommands(), fetchWorkspace());
  const api = await activateBackend(ctx).catch(err => {
    void vscode.window.showErrorMessage(
      `Cannot activate Argus extension: ${err.message}`
    );
    throw err;
  });

  globals = {
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
    // Public commands taht appear in the command palette and thus need to be listed in package.json.
    inspectWorkspace: { enabled: commands.launchArgus },

    // Private commands used internally, these should not appear in the command palette.
    blingObligation: { enabled: commands.blingObligation },
    openError: { enabled: commands.openError },
  };
}
