import { CharRange, Obligation, SerializedTree } from "./types";

// ----------------------------------------------------
// Communication between the extension and the webview.

export type Filename = string;

type FROM_EXT = "FROM_EXTENSION";
type FROM_WV = "FROM_WEBVIEW";

export type CommunicationDirection = FROM_EXT | FROM_WV;

export type CommonData = {
  // Data is specific to a single file.
  file: Filename;
};

export type ExtensionToWebViewMsg = CommonData & { type: FROM_EXT } & (
    | { command: "open-file" }
    | { command: "close-file" }
    | { command: "obligations"; obligations: Obligation[][] }
    | { command: "tree"; tree: SerializedTree[] }
  );

export type WebViewToExtensionMsg = CommonData & { type: FROM_WV } & (
    | { command: "obligations" }
    | { command: "tree"; predicate: Obligation }
    | { command: "add-highlight"; range: CharRange }
    | { command: "remove-highlight"; range: CharRange }
  );

// ------------------------------------------------------
// interface types between the extension and rustc plugin

// serde-compatible type
export type Result<T> = { Ok: T } | { Err: ArgusError };

export type BuildError = {
  type: "build-error";
  error: string;
};

export type ArgusError = {
  type: "analysis-error";
  error: string;
};

export interface ArgusOutput<T> {
  type: "output";
  value: T;
}

export type ArgusResult<T> = ArgusOutput<T> | ArgusError | BuildError;

// TODO: what we *really* want here is dependent typing...
// it might be achievable with TS, but too tired rn to think about that.
export type CallArgus = <T>(
  _args: ArgusArgs,
  _no_output?: boolean
) => Promise<ArgusResult<T>>;

export type ArgusArgs =
  | ["preload"] // => Obligation[][]
  | ["obligations", Filename] // => Obligation[][]
  | ["tree", Filename, string]; // => SerializedTree[]
