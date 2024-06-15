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
  FileInfo,
  Filename,
  PanoptesConfig,
  PanoptesToSystemCmds,
  PanoptesToSystemMsg,
  SystemToPanoptesCmds,
  SystemToPanoptesMsg,
  configToString,
  isPanoMsgAddHighlight,
  isPanoMsgRemoveHighlight,
  isPanoMsgTree,
} from "@argus/common/lib";
import { MessageHandlerData } from "@estruyf/vscode";
import _ from "lodash";
import os from "os";
import vscode from "vscode";

import { log } from "./logging";
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
  private readonly extensionUri: vscode.Uri;
  private disposables: vscode.Disposable[] = [];

  constructor(
    extensionUri: vscode.Uri,
    initialData: FileInfo[],
    target?: ErrorJumpTargetInfo,
    readonly cleanup: () => void = () => {},
    column: vscode.ViewColumn = vscode.ViewColumn.Beside
  ) {
    this.extensionUri = extensionUri;
    this.panel = this.makePanel(initialData, target, column);
  }

  get getPanel() {
    return this.panel;
  }

  makePanel(
    initialData: FileInfo[] = [],
    target?: ErrorJumpTargetInfo,
    column: vscode.ViewColumn = vscode.ViewColumn.Beside
  ): vscode.WebviewPanel {
    const panel = vscode.window.createWebviewPanel("argus", "Argus", column, {
      enableScripts: true,
      retainContextWhenHidden: true,
      enableFindWidget: true,
      localResourceRoots: [this.extensionUri],
    });

    // Set the webview's initial html content
    panel.webview.html = getHtmlForWebview(
      this.extensionUri,
      panel,
      initialData,
      target
    );

    // Listen for when the panel is disposed
    // This happens when the user closes the panel or when the panel is closed programatically
    panel.onDidDispose(
      () => {
        log("Disposing panel");
        globals.ctx.view = undefined;
        this.dispose();
      },
      null,
      this.disposables
    );

    // Handle messages from the webview
    panel.webview.onDidReceiveMessage(
      async message => await this.handleMessage(message),
      null,
      this.disposables
    );

    return panel;
  }

  public dispose() {
    // Clean up our resources
    this.cleanup();
    this.panel.dispose();
    while (this.disposables.length) {
      const x = this.disposables.pop();
      if (x) {
        x.dispose();
      }
    }
  }

  // Public API, using static methods >:(

  public async havoc() {
    this.messageWebview("havoc", {
      type: "FROM_EXTENSION",
      command: "havoc",
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

  public async openEditor(
    editor: RustEditor,
    signature: string,
    data: ObligationsInBody[]
  ) {
    console.debug("Sending open file message", editor.document.fileName);
    this.messageWebview("open-file", {
      type: "FROM_EXTENSION",
      command: "open-file",
      file: editor.document.fileName,
      signature,
      data,
    });
  }

  private async handleMessage(message: BlessedMessage<PanoptesToSystemCmds>) {
    const { requestId, payload } = message;
    if (isPanoMsgTree(payload)) {
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
}

function getHtmlForWebview(
  extensionUri: vscode.Uri,
  panel: vscode.WebviewPanel,
  initialData: FileInfo[] = [],
  target?: ErrorJumpTargetInfo
) {
  const panoptesDir = vscode.Uri.joinPath(extensionUri, "dist", "panoptes");

  const scriptUri = panel.webview.asWebviewUri(
    vscode.Uri.joinPath(panoptesDir, "dist", "panoptes.iife.js")
  );

  const styleUri = panel.webview.asWebviewUri(
    vscode.Uri.joinPath(panoptesDir, "dist", "style.css")
  );

  const codiconsUri = panel.webview.asWebviewUri(
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
