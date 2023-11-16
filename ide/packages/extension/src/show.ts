import { UnderlyingTree } from "@argus/common";
import * as vscode from "vscode";

import { globals } from "./lib";
import { asyncWithProgress } from "./logging";
import { ViewLoader } from "./viewloader";

// TODO: proof trees are specific to a single entry call to the trait solver.
//       It seems that we would want to use the debugger for a specific case,
//       and the user could click on a span to do that (similar to Flowistry)
//       or we could open argus with all calls to the solver and then allow the
//       user to click on a single one to get the proof tree.

export let displayAll = async (extensionPath: vscode.Uri) => {
  vscode.window.showInformationMessage("Hello World from Argus!");

  // Call Argus and get the proof tree
  let res = await asyncWithProgress(async () => {
    return await globals.backend<UnderlyingTree>([]);
  });

  if (res.type === "AnalysisError" || res.type == "BuildError") {
    vscode.window.showErrorMessage(res.error);
    return;
  }

  ViewLoader.createOrShow(res.value, extensionPath);
};
