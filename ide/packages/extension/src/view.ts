import {
  BodyHash,
  CharRange,
  ExprIdx,
  Obligation,
  ObligationHash,
  ObligationsInBody,
  SerializedTree,
} from "@argus/common/bindings";
import {
  ErrorJumpTargetInfo,
  ExtensionToWebViewMsg,
  Filename,
  ObligationOutput,
  OpenErrorPayload,
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

// TODO: instead of having a single view, with a static panel,
// we should have a view field on the Ctx, this makes all commands
// routed through the Ctx and we don't have to play the static
// shenanigans.
export class View {
  private panel: vscode.WebviewPanel;
  private isPanelDisposed: boolean;
  private readonly extensionUri: vscode.Uri;
  private disposables: vscode.Disposable[] = [];

  constructor(
    extensionUri: vscode.Uri,
    initialData: [Filename, ObligationsInBody[]][],
    target?: ErrorJumpTargetInfo,
    column: vscode.ViewColumn = vscode.ViewColumn.Beside
  ) {
    this.extensionUri = extensionUri;
    this.isPanelDisposed = true;
    // getPanel creates a new panel if it doesn't exist.
    this.panel = this.getPanel(initialData, target, column);
  }

  public getPanel(
    initialData: [Filename, ObligationsInBody[]][] = [],
    target?: ErrorJumpTargetInfo,
    column: vscode.ViewColumn = vscode.ViewColumn.Beside
  ): vscode.WebviewPanel {
    if (!this.isPanelDisposed) {
      return this.panel;
    }

    this.panel = vscode.window.createWebviewPanel("argus", "Argus", column, {
      enableScripts: true,
      localResourceRoots: [this.extensionUri],
    });
    this.isPanelDisposed = false;

    // Set the webview's initial html content
    this.panel.webview.html = this.getHtmlForWebview(initialData, target);

    // Listen for when the panel is disposed
    // This happens when the user closes the panel or when the panel is closed programatically
    this.panel.onDidDispose(
      () => {
        globals.ctx.view = undefined;
        this.dispose();
      },
      null,
      this.disposables
    );

    // Handle messages from the webview
    this.panel.webview.onDidReceiveMessage(
      async message => await this.handleMessage(message),
      null,
      this.disposables
    );

    return this.panel;
  }

  public dispose() {
    // Clean up our resources
    this.isPanelDisposed = true;
    this.panel.dispose();
    while (this.disposables.length) {
      const x = this.disposables.pop();
      if (x) {
        x.dispose();
      }
    }
  }

  // Public API, using static methods >:(

  public async reset(newData: [Filename, ObligationOutput[]][]) {
    this.messageWebview<[Filename, ObligationOutput[]][]>("invalidate", {
      type: "FROM_EXTENSION",
      command: "reset",
      data: newData,
    });
  }

  public async blingObligation(
    file: Filename,
    bodyIdx: BodyHash,
    exprIdx: ExprIdx,
    obligation: ObligationHash
  ) {
    this.messageWebview<Omit<OpenErrorPayload, "command">>("open-error", {
      type: "FROM_EXTENSION",
      command: "open-error",
      file,
      bodyIdx,
      exprIdx,
      hash: obligation,
    });
  }

  public async openEditor(editor: RustEditor, data: ObligationOutput[]) {
    this.messageWebview<Filename>("open-file", {
      type: "FROM_EXTENSION",
      command: "open-file",
      file: editor.document.fileName,
      data,
    });
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
    const obligations = await globals.ctx.getObligations(host);
    if (obligations !== undefined) {
      this.messageWebview<ObligationOutput[]>(requestId, {
        type: "FROM_EXTENSION",
        file: host,
        command: "obligations",
        obligations: obligations,
      });
    }
  }

  private async getTree(
    requestId: string,
    file: Filename,
    obl: Obligation,
    range: CharRange
  ) {
    const tree = await globals.ctx.getTree(file, obl, range);
    if (tree !== undefined) {
      this.messageWebview<SerializedTree>(requestId, {
        type: "FROM_EXTENSION",
        file,
        command: "tree",
        tree,
      });
    }
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

  private getHtmlForWebview(
    initialData: [Filename, ObligationsInBody[]][] = [],
    target?: ErrorJumpTargetInfo
  ) {
    const panoptesDir = vscode.Uri.joinPath(
      this.extensionUri,
      "dist",
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

    return `
      <!DOCTYPE html>
      <html lang="en">
      <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>Argus Inspector</title>
          <link rel="stylesheet" type="text/css" href=${styleUri}>
          <link rel="stylesheet" type="text/css" href=${codiconsUri}>

          <script>
            (function () {
              window.data = ${JSON.stringify(initialData)};
              window.target = ${JSON.stringify(target)};
            })()
          </script>
      </head>
      <body>
          <div id="root"></div>
          <script src="${scriptUri}"></script>
      </body>
      </html>
    `;
  }
}
