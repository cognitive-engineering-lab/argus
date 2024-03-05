import vscode from "vscode";

// Using the background selection color is sometimes a little too subtle.
export const rangeHighlight = vscode.window.createTextEditorDecorationType({
  backgroundColor: new vscode.ThemeColor("editor.selectionBackground"),
  borderRadius: "2px",
});

// TODO: these need to use the vscode specific colors editor.errorForeground, etc.

export const traitErrorDecorate = vscode.window.createTextEditorDecorationType({
  borderWidth: "0 0 1px 0",
  borderStyle: "solid",
  borderColor: new vscode.ThemeColor("editorError.border"),
});

export const ambigErrorDecorate = vscode.window.createTextEditorDecorationType({
  borderWidth: "0 0 1px 0",
  borderStyle: "solid",
  borderColor: new vscode.ThemeColor("editorWarning.border"),
});
