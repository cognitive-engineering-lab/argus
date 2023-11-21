import { WebViewToExtensionMsg } from "@argus/common";
// FIXME: why is the 'dist/' necessary? Elsewhere I can just use '@argus/common/types'.
import { CharRange, Obligation } from "@argus/common/dist/types";
import _ from "lodash";
import * as path from "path";
import * as vscode from "vscode";

import { globals } from "./lib";
import { asyncWithProgress, log } from "./logging";

const outDir = "dist";
const packageName = "panoptes";

function rootUri(extensionUri: vscode.Uri) {
  return vscode.Uri.joinPath(extensionUri, "..");
}

export let launchArgus = async (extensionPath: vscode.Uri) => {
  ViewLoader.createOrShow(extensionPath);
};

// ---------------------------------------------------------------------------
// TODO: lots of the highlight stuff is a major hack. This should really work 
// on the `visibleTextEditors`, and just an active editor (bc it doesn't stay 
// active).
// ---------------------------------------------------------------------------

// FIXME: these definitely need to change somehow.
const rangeHighlight = vscode.window.createTextEditorDecorationType({
  backgroundColor: new vscode.ThemeColor("editor.selectionBackground"),
  // color: new vscode.ThemeColor("editor.selectionForeground"),
  borderRadius: "2px",
});

export class ViewLoader {
  public static currentPanel: ViewLoader | undefined;
  private static readonly viewType = "react";
  private readonly _panel: vscode.WebviewPanel;
  private readonly _extensionPath: vscode.Uri;
  private _disposables: vscode.Disposable[] = [];

  private highlightRanges: CharRange[] = [];
  private currentEditor: vscode.TextEditor | undefined;

  public static createOrShow(extensionPath: vscode.Uri) {
    const column = vscode.window.activeTextEditor
      ? vscode.window.activeTextEditor.viewColumn
      : undefined;

    if (ViewLoader.currentPanel) {
      ViewLoader.currentPanel._panel.reveal(column);
    } else {
      ViewLoader.currentPanel = new ViewLoader(
        extensionPath,
        column || vscode.ViewColumn.Beside
      );
    }
  }

  private constructor(extensionPath: vscode.Uri, column: vscode.ViewColumn) {
    this._extensionPath = extensionPath;
    const root = rootUri(this._extensionPath);

    log(root);

    this._panel = vscode.window.createWebviewPanel("argus", "Argus", column, {
      enableScripts: true,
      localResourceRoots: [root],
    });

    // Set the webview's initial html content
    this._panel.webview.html = this._getHtmlForWebview();

    // Listen for when the panel is disposed
    // This happens when the user closes the panel or when the panel is closed programatically
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);

    // Handle messages from the webview
    this._panel.webview.onDidReceiveMessage(
      async message => await this.handleMessage(message),
      null,
      this._disposables
    );
  }

  public dispose() {
    ViewLoader.currentPanel = undefined;
    // Clean up our resources
    this._panel.dispose();
    while (this._disposables.length) {
      const x = this._disposables.pop();
      if (x) {
        x.dispose();
      }
    }
  }

  // ---------------------------------------------------

  private async handleMessage(message: WebViewToExtensionMsg) {
    const activeEditor = vscode.window.activeTextEditor;
    const openEditor = activeEditor || this.currentEditor; 

    if (openEditor === undefined) {
      vscode.window.showErrorMessage("No active editor");
      return;
    }

    this.currentEditor = openEditor;

    switch (message.command) {
      case "obligations": {
        this.getObligations(openEditor);
        return;
      }
      case "tree": {
        this.getTree(openEditor, message.line, message.column);
        return;
      }
      case "add-highlight": {
        this.addHighlightRange(openEditor, message.range);
        return;
      }
      case "remove-highlight": {
        this.removeHighlightRange(openEditor, message.range);
        return;
      }
      default: {
        vscode.window.showErrorMessage(`Message not understood ${message}`);
        return;
      }
    }
  }

  private async getObligations(editor: vscode.TextEditor) {
    log("Fetching obligations for file", editor.document.fileName);

    const res = await globals.backend<Obligation[][]>([
      "obligations",
      editor.document.fileName,
    ]);

    log("Result", res);

    if (res.type !== "output") {
      vscode.window.showErrorMessage(res.error);
      return;
    }

    const obligations = res.value;

    log("Returning obligations", obligations);

    this._panel.webview.postMessage({
      type: "FROM_EXTENSION",
      command: "obligations",
      obligations: obligations,
    });
  }

  private async getTree(editor: vscode.TextEditor, line: number, column: number) {
    throw new Error("Not implemented");
  }

  private async addHighlightRange(editor: vscode.TextEditor, range: CharRange) {
    // TODO: check the open file
    
    log("Adding highlight range", range);

    this.highlightRanges.push(range);
    await this.refreshHighlights(editor);
  }

  private async removeHighlightRange(editor: vscode.TextEditor, range: CharRange) {
    // TODO: check the open file

    log("Removing highlight range", range);

    this.highlightRanges = _.filter(
      this.highlightRanges,
      r => !_.isEqual(r, range)
    );
    await this.refreshHighlights(editor);
  }

  private async refreshHighlights(editor: vscode.TextEditor) {
    // TODO: check the open file

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

  private _getHtmlForWebview() {
    const root = rootUri(this._extensionPath);
    const buildDir = vscode.Uri.joinPath(root, packageName, outDir);

    const scriptUri = this._panel.webview.asWebviewUri(
      vscode.Uri.joinPath(buildDir, "panoptes.iife.js")
    );

    const styleUri = this._panel.webview.asWebviewUri(
      vscode.Uri.joinPath(buildDir, "style.css")
    );

    return `
      <!DOCTYPE html>
      <html lang="en">
      <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>Config View</title>
          <link rel="stylesheet" type="text/css" href=${styleUri}>
      </head>
      <body>
          <div id="root"></div>
          <script src="${scriptUri}"></script>
      </body>
      </html>
    `;
  }
}
