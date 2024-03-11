import vscode from "vscode";

// Range highlights are used to visually coordinate hovers in the webview.
export const rangeHighlight = vscode.window.createTextEditorDecorationType({
  backgroundColor: new vscode.ThemeColor("editor.selectionBackground"),
  borderRadius: "2px",
});
