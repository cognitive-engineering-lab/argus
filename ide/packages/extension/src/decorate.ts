import vscode from "vscode";

// TODO: all colors / decorations in this file need to be tweaked so they work
// will all themes in vscode.

// Using the background selection color is sometimes a little too subtle.
export const rangeHighlight = vscode.window.createTextEditorDecorationType({
  backgroundColor: new vscode.ThemeColor("editor.selectionBackground"),
  borderRadius: "2px",
});

export const traitErrorDecorate = vscode.window.createTextEditorDecorationType({
  borderWidth: "1px",
  borderStyle: "solid",
  overviewRulerColor: "blue",
  overviewRulerLane: vscode.OverviewRulerLane.Right,
  light: {
    // this color will be used in light color themes
    borderColor: "#F47174",
  },
  dark: {
    // this color will be used in dark color themes
    borderColor: "#CF6679",
  },
});
