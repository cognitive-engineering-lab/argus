import { UnderlyingTree } from "@argus/common";
import _ from "lodash";
import * as path from "path";
import * as vscode from "vscode";

import { log } from "./logging";

const outDir = "dist";
const packageName = "panoptes";

function rootUri(extensionUri: vscode.Uri) {
  return vscode.Uri.joinPath(extensionUri, "..");
}

export class ViewLoader {
  public static currentPanel: ViewLoader | undefined;

  private static readonly viewType = "react";

  private readonly _panel: vscode.WebviewPanel;
  private readonly _extensionPath: vscode.Uri;
  private _disposables: vscode.Disposable[] = [];

  public static createOrShow(data: UnderlyingTree, extensionPath: vscode.Uri) {
    const column = vscode.window.activeTextEditor
      ? vscode.window.activeTextEditor.viewColumn
      : undefined;

    // If we already have a panel, show it.
    // Otherwise, create a new panel.
    if (ViewLoader.currentPanel) {
      ViewLoader.currentPanel._panel.reveal(column);
    } else {
      ViewLoader.currentPanel = new ViewLoader(
        data,
        extensionPath,
        column || vscode.ViewColumn.Beside
      );
    }
  }

  private constructor(
    tree: UnderlyingTree,
    extensionPath: vscode.Uri,
    column: vscode.ViewColumn
  ) {
    this._extensionPath = extensionPath;
    const root = rootUri(this._extensionPath);

    log(root);

    this._panel = vscode.window.createWebviewPanel("argus", "Argus", column, {
      enableScripts: true,
      localResourceRoots: [ root ],
    });

    // Set the webview's initial html content
    this._panel.webview.html = this._getHtmlForWebview(tree);

    // Listen for when the panel is disposed
    // This happens when the user closes the panel or when the panel is closed programatically
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);

    // Handle messages from the webview
    this._panel.webview.onDidReceiveMessage(
      message => {
        switch (message.command) {
          case "alert":
            vscode.window.showErrorMessage(message.text);
            return;
        }
      },
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

  private _getHtmlForWebview(data: UnderlyingTree) {
    const root = rootUri(this._extensionPath);
    const buildDir = vscode.Uri.joinPath(root, packageName, outDir);

    const scriptUri = this._panel.webview.asWebviewUri(
      vscode.Uri.joinPath(buildDir, "panoptes.iife.js")
    );

    const styleUri = this._panel.webview.asWebviewUri(
      vscode.Uri.joinPath(buildDir, "style.css")
    );

          // <meta http-equiv="Content-Security-Policy" 
          //       content="default-src 'none'; 
          //                img-src https:; 
          //                script-src 'unsafe-eval' 'unsafe-inline' vscode-resource:; 
          //                style-src 'unsafe-eval' 'unsafe-inline' vscode-resource:;">

    return `
      <!DOCTYPE html>
      <html lang="en">
      <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>Config View</title>
          <link rel="stylesheet" type="text/css" href=${styleUri}>
          <script>window.initialData = ${JSON.stringify(data)};</script>
      </head>
      <body>
          <div id="root"></div>
          <script src="${scriptUri}"></script>
      </body>
      </html>
    `;
  }
}
