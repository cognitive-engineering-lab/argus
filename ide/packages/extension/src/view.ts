import os from "node:os";
import type {
  CharRange,
  Obligation,
  ObligationsInBody
} from "@argus/common/bindings";
import {
  ConfigConsts,
  type ErrorJumpTargetInfo,
  type FileInfo,
  type Filename,
  type PanoptesConfig,
  type PanoptesToSystemCmds,
  type PanoptesToSystemMsg,
  type SystemToPanoptesCmds,
  type SystemToPanoptesMsg,
  configToString,
  isPanoMsgAddHighlight,
  isPanoMsgRemoveHighlight,
  isPanoMsgTree
} from "@argus/common/lib";
import type { MessageHandlerData } from "@estruyf/vscode";
import vscode from "vscode";

import type { Ctx } from "./ctx";
import { log } from "./logging";
import type { RustEditor } from "./utils";

// Wraps around the MessageHandler data types from @estruyf/vscode.
type BlessedMessage<T extends PanoptesToSystemCmds> = {
  command: string;
  requestId: string;
  payload: PanoptesToSystemMsg<T>;
};

export class View {
  private panel: vscode.WebviewPanel;
  private disposables: vscode.Disposable[] = [];

  constructor(
    private readonly ctx: Ctx,
    initialData: FileInfo[],
    target?: ErrorJumpTargetInfo,
    readonly cleanup: () => void = () => {},
    column: vscode.ViewColumn = vscode.ViewColumn.Beside
  ) {
    this.panel = this.makePanel(initialData, target, column);
  }

  get getPanel() {
    return this.panel;
  }

  private makePanel(
    initialData: FileInfo[] = [],
    target?: ErrorJumpTargetInfo,
    column: vscode.ViewColumn = vscode.ViewColumn.Beside
  ): vscode.WebviewPanel {
    const panel = vscode.window.createWebviewPanel("argus", "Argus", column, {
      enableScripts: true,
      retainContextWhenHidden: true,
      enableFindWidget: true,
      localResourceRoots: [this.ctx.ictxt.extCtx.extensionUri]
    });

    // Set the webview's initial html content
    panel.webview.html = getHtmlForWebview(
      this.ctx.ictxt.extCtx.extensionUri,
      panel,
      initialData,
      target
    );

    // Listen for when the panel is disposed
    // This happens when the user closes the panel or when the panel is closed programatically
    this.disposables.push(
      panel.onDidDispose(() => {
        log("Disposing panel");
        this.cleanup();
        this.panel.dispose();
        while (this.disposables.length) {
          const x = this.disposables.pop();
          if (x) {
            x.dispose();
          }
        }
      })
    );

    // Handle messages from the webview
    this.disposables.push(
      panel.webview.onDidReceiveMessage(async message => {
        try {
          await this.handleMessage(message);
        } catch (e: any) {
          log(`Handler threw error ${e.message}`);
        }
      })
    );

    return panel;
  }

  public async havoc() {
    messageWebview(this.panel.webview, "havoc", {
      type: "FROM_EXTENSION"
    });
  }

  public async blingObligation({
    file,
    bodyIdx,
    exprIdx,
    hash
  }: ErrorJumpTargetInfo) {
    messageWebview(this.panel.webview, "open-error", {
      type: "FROM_EXTENSION",
      file,
      bodyIdx,
      exprIdx,
      hash
    });
  }

  public async openEditor(
    editor: RustEditor,
    signature: string,
    data: ObligationsInBody[]
  ) {
    messageWebview(this.panel.webview, "open-file", {
      type: "FROM_EXTENSION",
      file: editor.document.fileName,
      signature,
      data
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
      return this.ctx.addHighlightRange(payload.file, payload.range);
    } else if (isPanoMsgRemoveHighlight(payload)) {
      return this.ctx.removeHighlightRange(payload.file, payload.range);
    }
  }

  private async getTree(
    requestId: string,
    file: Filename,
    obl: Obligation,
    range: CharRange
  ) {
    const tree = await this.ctx.getTree(file, obl, range);
    if (tree !== undefined) {
      messageWebview(
        this.panel.webview,
        "tree",
        {
          type: "FROM_EXTENSION",
          file,
          tree
        },
        requestId
      );
    }
  }

  sendPinRequest() {
    messageWebview(this.panel.webview, "pin", {
      type: "FROM_EXTENSION"
    });
  }

  sendUnpinRequest() {
    messageWebview(this.panel.webview, "unpin", {
      type: "FROM_EXTENSION"
    });
  }
}

function messageWebview<T extends SystemToPanoptesCmds>(
  webview: vscode.Webview,
  command: T,
  msg: Omit<SystemToPanoptesMsg<T>, "command">,
  requestId: string = command
) {
  webview.postMessage({
    requestId,
    payload: { command, ...msg }
  } as MessageHandlerData<SystemToPanoptesMsg<T>>);
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
      vscodeVersion: vscode.version
    }
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
