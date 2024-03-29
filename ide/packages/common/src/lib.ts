import {
  BodyHash,
  CharRange,
  ExprIdx,
  Obligation,
  ObligationHash,
  ObligationsInBody,
  SerializedTree,
} from "./bindings";

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

export type SystemReturn<T extends PanoptesToSystemCmds> = T extends "tree"
  ? { tree?: SerializedTree }
  : T extends "obligations"
  ? { obligations: ObligationsInBody[] }
  : {};

export interface OpenErrorPayload {
  command: "open-error";
  bodyIdx: BodyHash;
  exprIdx: ExprIdx;
  hash: ObligationHash;
}

export type PayloadTypes = {
  "open-error": Omit<OpenErrorPayload, "command">;
  "open-file": { data: ObligationsInBody[] };
  obligations: { obligations: ObligationsInBody[] };
  tree: { tree?: SerializedTree };
};

export type SystemToPanoptesCmds =
  | "reset"
  | "open-file"
  | "open-error"
  | "obligations"
  | "tree";

export type SystemToPanoptesMsg<T extends SystemToPanoptesCmds> = {
  command: T;
  type: FROM_EXT;
} & (T extends "reset"
  ? { data: [Filename, ObligationsInBody[]][] }
  : CommonData &
      (T extends "open-file"
        ? { data: ObligationsInBody[] }
        : T extends "open-error"
        ? {
            bodyIdx: BodyHash;
            exprIdx: ExprIdx;
            hash: ObligationHash;
          }
        : T extends "obligations"
        ? { obligations: ObligationsInBody[] }
        : T extends "tree"
        ? { tree?: SerializedTree }
        : never));

export type PanoptesToSystemCmds =
  | "obligations"
  | "tree"
  | "add-highlight"
  | "remove-highlight";
export type PanoptesToSystemMsg<T extends PanoptesToSystemCmds> = CommonData & {
  command: T;
  type: FROM_WV;
} & (T extends "obligations"
    ? {}
    : T extends "tree"
    ? { predicate: Obligation; range: CharRange }
    : T extends "add-highlight" | "remove-highlight"
    ? { range: CharRange }
    : never);

// ------------------------------------------------------
// Interface between the system and rustc plugin

export type ArgusCliOptions = "preload" | "tree" | "obligations";

export type ArgusArgs<T extends ArgusCliOptions> = T extends "preload"
  ? ["preload"]
  : T extends "obligations"
  ? ["obligations", Filename]
  : T extends "tree"
  ? ["tree", Filename, string, number, number, number, number, boolean]
  : never;

export type ArgusReturn<T extends ArgusCliOptions> = T extends "preload"
  ? void
  : T extends "tree"
  ? Array<SerializedTree | undefined>
  : T extends "obligations"
  ? ObligationsInBody[]
  : never;

// serde-compatible type
export type Result<T> = { Ok: T } | { Err: ArgusError };

export type ArgusError =
  | { type: "analysis-error"; error: string }
  | { type: "build-error"; error: string };

export interface ArgusOutput<T> {
  type: "output";
  value: T;
}

export type ArgusResult<T extends ArgusCliOptions> =
  | ArgusOutput<ArgusReturn<T>>
  | ArgusError;

// TODO: what we really want here is dependent typing ... it
// might be achievable with TS, but too tired rn to think about that.
export type CallArgus = <T extends ArgusCliOptions>(
  _args: ArgusArgs<T>,
  _no_output?: boolean
) => Promise<ArgusResult<T>>;

// Type predicates (these shouldn't really exist ...)

export function isSysMsgOpenError(
  msg: SystemToPanoptesMsg<SystemToPanoptesCmds>
): msg is SystemToPanoptesMsg<"open-error"> {
  return msg.command === "open-error";
}

export function isSysMsgOpenFile(
  msg: SystemToPanoptesMsg<SystemToPanoptesCmds>
): msg is SystemToPanoptesMsg<"open-file"> {
  return msg.command === "open-file";
}

export function isSysMsgReset(
  msg: SystemToPanoptesMsg<SystemToPanoptesCmds>
): msg is SystemToPanoptesMsg<"reset"> {
  return msg.command === "reset";
}

export function isPanoMsgTree(
  msg: PanoptesToSystemMsg<PanoptesToSystemCmds>
): msg is PanoptesToSystemMsg<"tree"> {
  return msg.command === "tree";
}

export function isPanoMsgObligations(
  msg: PanoptesToSystemMsg<PanoptesToSystemCmds>
): msg is PanoptesToSystemMsg<"obligations"> {
  return msg.command === "obligations";
}

export function isPanoMsgAddHighlight(
  msg: PanoptesToSystemMsg<PanoptesToSystemCmds>
): msg is PanoptesToSystemMsg<"add-highlight"> {
  return msg.command === "add-highlight";
}

export function isPanoMsgRemoveHighlight(
  msg: PanoptesToSystemMsg<PanoptesToSystemCmds>
): msg is PanoptesToSystemMsg<"remove-highlight"> {
  return msg.command === "remove-highlight";
}
