import vscode from "vscode";

// Using the background selection color is sometimes a little too subtle.
export const rangeHighlight = vscode.window.createTextEditorDecorationType({
  backgroundColor: "var(--vscode-selection-background)",
  borderRadius: "2px",
});

export const traitErrorDecorate = vscode.window.createTextEditorDecorationType({
  textDecoration: "underline wavy var(--vscode-editorError-foreground)",
});

export const ambigErrorDecorate = vscode.window.createTextEditorDecorationType({
  textDecoration: "underline wavy var(--vscode-editorWarning-foreground)",
});

export const exprRangeDecorate = vscode.window.createTextEditorDecorationType({
  textDecoration: "underline wavy var(--vscode-editorInfo-foreground)",
});
