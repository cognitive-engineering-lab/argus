import {
  BodyHash,
  CharRange,
  ExprIdx,
  Obligation,
  ObligationHash,
  ObligationsInBody,
  SerializedTree,
} from "./bindings";

// TODO: we should probably just get rid of these,
// don't think they'll be needed anymore.
export type ObligationOutput = ObligationsInBody;
export type TreeOutput = SerializedTree | undefined;
export type ArgusOutputs = ObligationOutput | TreeOutput;

export interface ErrorJumpTargetInfo {
  file: Filename;
  bodyIdx: BodyHash;
  exprIdx: ExprIdx;
  hash: ObligationHash;
}

// ----------------------------------------------------
// Interface between the webview and extension

export type Filename = string;

type FROM_EXT = "FROM_EXTENSION";
type FROM_WV = "FROM_WEBVIEW";

export type CommunicationDirection = FROM_EXT | FROM_WV;

export type CommonData = {
  // Data is specific to a single file.
  file: Filename;
};

export type ExtensionReturn<T extends ExtensionToWebViewMsg["command"]> =
  T extends "tree"
    ? { tree: TreeOutput }
    : T extends "obligations"
    ? { obligations: ObligationOutput[] }
    : {};

export interface OpenErrorPayload {
  command: "open-error";
  bodyIdx: BodyHash;
  exprIdx: ExprIdx;
  hash: ObligationHash;
}

export type PayloadTypes = {
  "open-error": Omit<OpenErrorPayload, "command">;
  "open-file": { data: ObligationOutput[] };
  obligations: { obligations: ObligationOutput[] };
  tree: { tree: TreeOutput };
};

export type ExtensionToWebViewMsg = { type: FROM_EXT } & (
  | { command: "reset"; data: [Filename, ObligationsInBody[]][] }
  | (CommonData &
      (
        | OpenErrorPayload
        | { command: "open-file"; data: ObligationOutput[] }
        | { command: "obligations"; obligations: ObligationOutput[] }
        | { command: "tree"; tree: TreeOutput }
      ))
);

export type WebViewToExtensionMsg = CommonData & { type: FROM_WV } & (
    | { command: "obligations" }
    | {
        command: "tree";
        predicate: Obligation;
        range: CharRange;
      }
    | { command: "add-highlight"; range: CharRange }
    | { command: "remove-highlight"; range: CharRange }
  );

// ------------------------------------------------------
// Interface between the extension and rustc plugin

// serde-compatible type
export type Result<T> = { Ok: T } | { Err: ArgusError };

export type ArgusError =
  | { type: "analysis-error"; error: string }
  | { type: "build-error"; error: string };

export interface ArgusOutput<T> {
  type: "output";
  value: T;
}

export type ArgusResult<T> = ArgusOutput<T> | ArgusError;

// TODO: what we really want here is dependent typing ... it
// might be achievable with TS, but too tired rn to think about that.
export type CallArgus = <T>(
  _args: ArgusArgs,
  _no_output?: boolean
) => Promise<ArgusResult<T>>;

export type ArgusArgs =
  | ["preload"] // => void
  | ["obligations", Filename] // => ObligationsInBody[]
  // NOTE: the hashes need to remain a string, otherwise JS cuts off the higher bits on bignums.
  | ["tree", Filename, string, number, number, number, number, boolean]; // => [SerializedTree | undefined]
