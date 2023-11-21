import { Obligation, SerializedTree, CharRange } from "./types";

// ----------------------------------------------------
// Communication between the extension and the webview.

export type ExtensionToWebViewMsg =
  | { command: "none" }
  | { command: "obligations"; obligations: Obligation[][] }
  | { command: "tree"; tree: SerializedTree[] }
  ;

export type WebViewToExtensionMsg =
  | { command: "obligations" }
  | { command: "tree"; line: number; column: number }
  | { command: "add-highlight"; range: CharRange }
  | { command: "remove-highlight"; range: CharRange }
  ;
