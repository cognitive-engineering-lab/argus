import {
  ExtensionToWebViewMsg,
  Filename,
  WebViewToExtensionMsg,
} from "@argus/common";
// FIXME: some build thingy is wrong. The 'dist/' should not be necessary?
// Elsewhere I can just use '@argus/common/types', but TS can't find the type decls.
import {
  CharRange,
  Obligation,
  ObligationOutput,
  SerializedTree,
  TreeOutput,
} from "@argus/common/dist/types";
import _ from "lodash";
import path from "path";
import vscode from "vscode";

import { showErrorDialog } from "./errors";
import { globals } from "./lib";
import { log } from "./logging";

const outDir = "dist";
const packageName = "panoptes";

function rootUri(extensionUri: vscode.Uri) {
  return vscode.Uri.joinPath(extensionUri, "..");
}

export let launchArgus = async (extensionPath: vscode.Uri) => {
  ViewLoader.createOrShow(extensionPath);
};

// Using the background selection color is sometimes a little too subtle.
const rangeHighlight = vscode.window.createTextEditorDecorationType({
  backgroundColor: new vscode.ThemeColor("editor.selectionBackground"),
  borderRadius: "2px",
});

export class ViewLoader {
  public static currentPanel: ViewLoader | undefined;

  private readonly panel: vscode.WebviewPanel;
  private readonly extensionPath: vscode.Uri;

  private disposables: vscode.Disposable[] = [];
  private highlightRanges: CharRange[] = [];

  public static createOrShow(extensionPath: vscode.Uri) {
    const column = vscode.window.activeTextEditor
      ? vscode.window.activeTextEditor.viewColumn
      : undefined;

    if (ViewLoader.currentPanel) {
      ViewLoader.currentPanel.panel.reveal(column);
    } else {
      ViewLoader.currentPanel = new ViewLoader(
        extensionPath,
        column || vscode.ViewColumn.Beside
      );
    }
  }

  private constructor(extensionPath: vscode.Uri, column: vscode.ViewColumn) {
    this.extensionPath = extensionPath;
    const root = rootUri(this.extensionPath);

    this.panel = vscode.window.createWebviewPanel("argus", "Argus", column, {
      enableScripts: true,
      localResourceRoots: [root],
    });

    // Set the webview's initial html content
    this.panel.webview.html = this.getHtmlForWebview();

    // Listen for when the panel is disposed
    // This happens when the user closes the panel or when the panel is closed programatically
    this.panel.onDidDispose(() => this.dispose(), null, this.disposables);

    // Handle messages from the webview
    this.panel.webview.onDidReceiveMessage(
      async message => await this.handleMessage(message),
      null,
      this.disposables
    );

    // TODO: add / remove files when they are opened / closed. I'm unsure
    // how we actually want to do this. Someone can often have lots of open files,
    // which I'm sure we don't want to imitate, but only having visible files can also
    // get annoying if someone navigates away then wants to come back.
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

  // ---------------------------------------------------

  private async handleMessage(message: WebViewToExtensionMsg) {
    switch (message.command) {
      case "obligations": {
        this.getObligations(message.file);
        return;
      }
      case "tree": {
        this.getTree(message.file, message.predicate, message.range);
        return;
      }
      case "add-highlight": {
        this.addHighlightRange(message.file, message.range);
        return;
      }
      case "remove-highlight": {
        this.removeHighlightRange(message.file, message.range);
        return;
      }
      default: {
        vscode.window.showErrorMessage(`Message not understood ${message}`);
        return;
      }
    }
  }

  private async getObligations(host: Filename) {
    log("Fetching obligations for file", host);

    const res = await globals.backend<ObligationOutput[]>([
      "obligations",
      host,
    ]);

    log("Result", res);

    if (res.type !== "output") {
      vscode.window.showErrorMessage(res.error);
      return;
    }

    const obligations = res.value;

    log("Returning obligations", obligations);

    this.messageWebview({
      type: "FROM_EXTENSION",
      file: host,
      command: "obligations",
      obligations: obligations,
    });
  }

  private async getTree(host: Filename, obl: Obligation, range: CharRange) {
    log("Fetching tree for file", host, obl.hash, obl);

    const res = await globals.backend<TreeOutput[]>([
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
    // it is possible that no tree is returned. (This is but we're working on it.)
    const tree = _.filter(res.value, t => t !== undefined) as Array<
      SerializedTree | undefined
    >;
    const tree0 = tree[0];

    this.messageWebview({
      type: "FROM_EXTENSION",
      file: host,
      command: "tree",
      tree: tree0,
    });
  }

  private async addHighlightRange(host: Filename, range: CharRange) {
    log("Adding highlight range", host, range);

    const editor = this.getEditorByName(host);
    if (editor === undefined) {
      showErrorDialog(`No editor for file ${host}`);
    } else {
      this.highlightRanges.push(range);
      await this.refreshHighlights(editor);
    }
  }

  private async removeHighlightRange(host: Filename, range: CharRange) {
    log("Removing highlight range", host, range);

    const editor = this.getEditorByName(host);
    if (editor === undefined) {
      showErrorDialog(`No editor for file ${host}`);
    } else {
      this.highlightRanges = _.filter(
        this.highlightRanges,
        r => !_.isEqual(r, range)
      );
      await this.refreshHighlights(editor);
    }
  }

  private async refreshHighlights(editor: vscode.TextEditor) {
    editor.setDecorations(
      rangeHighlight,
      _.map(this.highlightRanges, (r: CharRange) => {
        return new vscode.Range(
          new vscode.Position(r.start.line, r.start.column),
          new vscode.Position(r.end.line, r.end.column)
        );
      })
    );
  }

  // --------------------------------
  // Utilities

  private messageWebview(msg: ExtensionToWebViewMsg) {
    this.panel.webview.postMessage(msg);
  }

  private getEditorByName(name: Filename): vscode.TextEditor | undefined {
    return _.find(
      vscode.window.visibleTextEditors,
      e => e.document.fileName === name
    );
  }

  private getHtmlForWebview() {
    const root = rootUri(this.extensionPath);
    const buildDir = vscode.Uri.joinPath(root, packageName, outDir);

    const scriptUri = this.panel.webview.asWebviewUri(
      vscode.Uri.joinPath(buildDir, "panoptes.iife.js")
    );

    const styleUri = this.panel.webview.asWebviewUri(
      vscode.Uri.joinPath(buildDir, "style.css")
    );

    const codiconsUri = this.panel.webview.asWebviewUri(
      vscode.Uri.joinPath(
        root,
        packageName,
        "node_modules",
        "@vscode/codicons",
        "dist",
        "codicon.css"
      )
    );

    let initialFiles: Filename[] = _.map(
      vscode.window.visibleTextEditors,
      e => e.document.fileName
    );

    return `
      <!DOCTYPE html>
      <html lang="en">
      <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>Config View</title>
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
