import {
  BodyHash,
  CharRange,
  ExprIdx,
  Obligation,
  ObligationHash,
  ObligationsInBody,
} from "@argus/common/bindings";
import {
  ConfigConsts,
  ErrorJumpTargetInfo,
  Filename,
  PanoptesConfig,
  PanoptesToSystemCmds,
  PanoptesToSystemMsg,
  SystemToPanoptesCmds,
  SystemToPanoptesMsg,
  configToString,
  isPanoMsgAddHighlight,
  isPanoMsgObligations,
  isPanoMsgRemoveHighlight,
  isPanoMsgTree,
} from "@argus/common/lib";
import { MessageHandlerData } from "@estruyf/vscode";
import _ from "lodash";
import os from "os";
import vscode from "vscode";

import { globals } from "./main";
import { RustEditor } from "./utils";

// Wraps around the MessageHandler data types from @estruyf/vscode.
type BlessedMessage<T extends PanoptesToSystemCmds> = {
  command: string;
  requestId: string;
  payload: PanoptesToSystemMsg<T>;
};

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
      retainContextWhenHidden: true,
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

  public async reset(newData: [Filename, ObligationsInBody[]][]) {
    this.messageWebview("reset", {
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
    this.messageWebview("open-error", {
      type: "FROM_EXTENSION",
      command: "open-error",
      file,
      bodyIdx,
      exprIdx,
      hash: obligation,
    });
  }

  public async openEditor(editor: RustEditor, data: ObligationsInBody[]) {
    console.debug("Sending open file message", editor.document.fileName);
    this.messageWebview("open-file", {
      type: "FROM_EXTENSION",
      command: "open-file",
      file: editor.document.fileName,
      data,
    });
  }

  private async handleMessage(message: BlessedMessage<PanoptesToSystemCmds>) {
    const { requestId, payload } = message;

    if (isPanoMsgObligations(payload)) {
      return this.getObligations(requestId, payload.file);
    } else if (isPanoMsgTree(payload)) {
      return this.getTree(
        requestId,
        payload.file,
        payload.predicate,
        payload.range
      );
    } else if (isPanoMsgAddHighlight(payload)) {
      return globals.ctx.addHighlightRange(payload.file, payload.range);
    } else if (isPanoMsgRemoveHighlight(payload)) {
      return globals.ctx.removeHighlightRange(payload.file, payload.range);
    }
  }

  private async getObligations(requestId: string, host: Filename) {
    const obligations = await globals.ctx.getObligations(host);
    if (obligations !== undefined) {
      this.messageWebview(requestId, {
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
      this.messageWebview(requestId, {
        type: "FROM_EXTENSION",
        file,
        command: "tree",
        tree,
      });
    }
  }

  private messageWebview<T extends SystemToPanoptesCmds>(
    requestId: string,
    msg: SystemToPanoptesMsg<T>
  ) {
    this.panel.webview.postMessage({
      requestId: requestId,
      payload: msg,
    } as MessageHandlerData<SystemToPanoptesMsg<T>>);
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

    const config: PanoptesConfig = {
      type: "VSCODE_BACKING",
      data: initialData,
      target,
      spec: {
        osPlatform: os.platform(),
        osRelease: os.release(),
        vscodeVersion: vscode.version,
      },
    };
    const configStr = configToString(config);

    return `
      <!DOCTYPE html>
      <html lang="en">
      <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>Argus Inspector</title>
          <link rel="stylesheet" type="text/css" href=${styleUri}>
          <link rel="stylesheet" type="text/css" href=${codiconsUri}>
      </head>
      <body>
          <div class=${ConfigConsts.EMBED_NAME} data-config=${configStr}></div>
          <script src="${scriptUri}"></script>
      </body>
      </html>
    `;
  }
}
