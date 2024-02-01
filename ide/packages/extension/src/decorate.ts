import vscode from "vscode";

// TODO: all colors / decorations in this file need to be tweaked so they work
// will all themes in vscode.

// Using the background selection color is sometimes a little too subtle.
export const rangeHighlight = vscode.window.createTextEditorDecorationType({
  backgroundColor: new vscode.ThemeColor("editor.selectionBackground"),
  borderRadius: "2px",
});

export const traitErrorDecorate = vscode.window.createTextEditorDecorationType({
  light: {
    textDecoration: "underline wavy #FF007F",
  },
  dark: {
    textDecoration: "underline wavy white",
  },
});

export const ambigErrorDecorate = vscode.window.createTextEditorDecorationType({
  light: {
    textDecoration: "underline wavy  #D1A023",
  },
  dark: {
    textDecoration: "underline wavy white",
  },
});

export const exprRangeDecorate = vscode.window.createTextEditorDecorationType({
  light: {
    textDecoration: "underline wavy #008DD1",
  },
  dark: {
    textDecoration: "underline wavy #CAF0F8",
  },
});
