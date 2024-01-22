import {
  CharRange,
  Obligation,
  ObligationHash,
  ObligationOutput,
  SerializedTree,
  TreeOutput,
} from "@argus/common/bindings";
import {
  ExtensionToWebViewMsg,
  Filename,
  WebViewToExtensionMsg,
} from "@argus/common/lib";
import { MessageHandlerData } from "@estruyf/vscode";
import _ from "lodash";
import vscode from "vscode";

import { log } from "./logging";
import { globals } from "./main";
import { RustEditor } from "./utils";

// Wraps around the MessageHandler data types from @estruyf/vscode.
type BlessedMessage = {
  command: string;
  requestId: string;
  payload: WebViewToExtensionMsg;
};

export class ViewLoader {
  private static currentPanel: ViewLoader | undefined;
  private readonly panel: vscode.WebviewPanel;
  private readonly extensionUri: vscode.Uri;
  private disposables: vscode.Disposable[] = [];

  public static createOrShow(extensionUri: vscode.Uri) {
    if (ViewLoader.currentPanel) {
      ViewLoader.currentPanel.panel.reveal();
    } else {
      ViewLoader.currentPanel = new ViewLoader(
        extensionUri,
        vscode.ViewColumn.Beside
      );
    }
  }

  private constructor(extensionUri: vscode.Uri, column: vscode.ViewColumn) {
    console.log(`Creating view in ${extensionUri}`);
    this.extensionUri = extensionUri;
    this.panel = vscode.window.createWebviewPanel("argus", "Argus", column, {
      enableScripts: true,
      localResourceRoots: [extensionUri],
    });

    // Set the webview's initial html content
    this.panel.webview.html = this.getHtmlForWebview(
      globals.ctx.visibleEditors
    );

    // Listen for when the panel is disposed
    // This happens when the user closes the panel or when the panel is closed programatically
    this.panel.onDidDispose(() => this.dispose(), null, this.disposables);

    // Handle messages from the webview
    this.panel.webview.onDidReceiveMessage(
      async message => await this.handleMessage(message),
      null,
      this.disposables
    );
  }

  public dispose() {
    ViewLoader.currentPanel = undefined;
    // Clean up our resources
    this.panel.dispose();
    while (this.disposables.length) {
      const x = this.disposables.pop();
      if (x) {
        x.dispose();
      }
    }
  }

  private async handleMessage(message: BlessedMessage) {
    const { command, requestId, payload } = message;

    if (command !== payload.command) {
      log(
        `
        Command mismatch 
          expected: ${payload.command} 
          but got: ${command}
        `
      );
      return;
    }

    switch (payload.command) {
      case "obligations": {
        this.getObligations(requestId, payload.file);
        return;
      }
      case "tree": {
        this.getTree(requestId, payload.file, payload.predicate, payload.range);
        return;
      }

      // These messages don't require a response.
      // TODO: but they need to interact with the Ctx.
      //
      case "add-highlight": {
        globals.ctx.addHighlightRange(payload.file, payload.range);
        return;
      }
      case "remove-highlight": {
        globals.ctx.removeHighlightRange(payload.file, payload.range);
        return;
      }
      default: {
        log(`Message not understood ${message}`);
        return;
      }
    }
  }

  private async getObligations(requestId: string, host: Filename) {
    const res = await globals.ctx.backend<ObligationOutput[]>([
      "obligations",
      host,
    ]);

    if (res.type !== "output") {
      vscode.window.showErrorMessage(res.error);
      return;
    }
    const obligations = res.value;

    // For each of the returned bodies, we need to register the trait errors
    // in the editor context. TODO: register ambiguity errors when we have them.
    const traitErrors = _.flatMap(obligations, oib => oib.traitErrors);
    const ambiguityErrors = _.flatMap(obligations, oib => oib.ambiguityErrors);

    globals.ctx.setTraitErrors(host, traitErrors);
    globals.ctx.setAmbiguityErrors(host, ambiguityErrors);

    this.messageWebview<ObligationOutput[]>(requestId, {
      type: "FROM_EXTENSION",
      file: host,
      command: "obligations",
      obligations: obligations,
    });
  }

  private async getTree(
    requestId: string,
    host: Filename,
    obl: Obligation,
    range: CharRange
  ) {
    const res = await globals.ctx.backend<TreeOutput[]>([
      "tree",
      host,
      obl.hash,
      range.start.line,
      range.start.column,
      range.end.line,
      range.end.column,
    ]);

    if (res.type !== "output") {
      vscode.window.showErrorMessage(res.error);
      return;
    }

    // NOTE: the returned value should be an array of a single tree, however,
    // it is possible that no tree is returned. (Yes, but I'm working on it.)
    const tree = _.filter(res.value, t => t !== undefined) as Array<
      SerializedTree | undefined
    >;
    const tree0 = tree[0];

    this.messageWebview<SerializedTree>(requestId, {
      type: "FROM_EXTENSION",
      file: host,
      command: "tree",
      tree: tree0,
    });
  }

  public static async blingObligation(
    file: Filename,
    obligation: ObligationHash
  ) {
    this.currentPanel?.messageWebview<ObligationHash>("bling", {
      type: "FROM_EXTENSION",
      command: "bling",
      file,
      oblHash: obligation,
    });
  }

  // FIXME: the type T here is wrong, it should be a response message similar to
  // how the webview encodes the return value.
  private messageWebview<T>(requestId: string, msg: ExtensionToWebViewMsg) {
    this.panel.webview.postMessage({
      command: msg.command,
      requestId: requestId,
      payload: msg,
    } as MessageHandlerData<T>);
  }

  private getHtmlForWebview(openEditors: RustEditor[] = []) {
    const panoptesDir = vscode.Uri.joinPath(
      this.extensionUri,
      "node_modules",
      "@argus",
      "panoptes"
    );

    const scriptUri = this.panel.webview.asWebviewUri(
      vscode.Uri.joinPath(panoptesDir, "dist", "panoptes.iife.js")
    );

    const styleUri = this.panel.webview.asWebviewUri(
      vscode.Uri.joinPath(panoptesDir, "dist", "style.css")
    );

    const codiconsUri = this.panel.webview.asWebviewUri(
      vscode.Uri.joinPath(
        panoptesDir,
        "node_modules",
        "@vscode/codicons",
        "dist",
        "codicon.css"
      )
    );

    const initialFiles: Filename[] = _.map(
      openEditors,
      e => e.document.fileName
    );

    return `
      <!DOCTYPE html>
      <html lang="en">
      <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>Argus Inspector</title>
          <link rel="stylesheet" type="text/css" href=${styleUri}>
          <link rel="stylesheet" type="text/css" href=${codiconsUri}>
          <script>window.initialData = ${JSON.stringify(initialFiles)};</script>
      </head>
      <body>
          <div id="root"></div>
          <script src="${scriptUri}"></script>
      </body>
      </html>
    `;
  }
}
