import type { CancelablePromise as CPromise } from "cancelable-promise";
import newGithubIssueUrl from "new-github-issue-url";

import type {
  BodyBundle,
  BodyHash,
  CharRange,
  DefLocation,
  ExprIdx,
  Obligation,
  ObligationHash,
  ObligationsInBody,
  SerializedTree
} from "./bindings";

export interface ErrorJumpTargetInfo {
  file: Filename;
  bodyIdx: BodyHash;
  exprIdx: ExprIdx;
  hash: ObligationHash;
}

export const ConfigConsts = {
  PANOPTES_NAME: "panoptes",
  EMBED_NAME: "argus-embed"
};

// ----------------------------------------------------
// Panoptes initial configuration for a single webview

export type PanoptesOptionalData = {
  scroll?: boolean;
  target?: ErrorJumpTargetInfo;
  evalMode?: EvaluationMode;
  rankMode?: SortStrategy;
};

export type SystemSpec = Omit<IssueOptions, "logText">;
export type EvaluationMode = "release" | "evaluate";
export type SortStrategy = "inertia" | "depth" | "vars";

export interface FileInfo {
  fn: Filename;
  data: ObligationsInBody[];
  signature?: string;
}

export type PanoptesConfig = PanoptesOptionalData &
  (
    | {
        type: "VSCODE_BACKING";
        spec: SystemSpec;
        data: FileInfo[];
      }
    | {
        type: "WEB_BUNDLE";
        closedSystem: BodyBundle[];
      }
  );

export function configToString(config: PanoptesConfig) {
  return encodeURI(JSON.stringify(config));
}

export function maybeStringToConfig(str?: string): PanoptesConfig | undefined {
  return str ? JSON.parse(decodeURI(str)) : undefined;
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
  // Destroy all obligation and tree data
  | "havoc"
  // (Un-)Pin mini-buffer data for inspection
  | "pin"
  | "unpin"
  // Open the current file into the webview workspace
  | "open-file"
  // Highlight and error and scroll it into view
  | "open-error"
  // Send the requested tree to the webview
  | "tree";

export type SystemToPanoptesMsg<T extends SystemToPanoptesCmds> = {
  command: T;
  type: FROM_EXT;
} & (T extends "havoc" | "pin" | "unpin"
  ? {}
  : // ^^^ NOTE ^^^
    // Havoc and Pin are global operations and don't require View state
    CommonData &
      (T extends "open-file"
        ? { data: ObligationsInBody[]; signature: string }
        : T extends "open-error"
          ? {
              bodyIdx: BodyHash;
              exprIdx: ExprIdx;
              hash: ObligationHash;
            }
          : T extends "tree"
            ? { tree?: SerializedTree }
            : never));

export type PanoptesToSystemCmds =
  // Request obligations associated with the current file
  | "obligations"
  // Request the proof tree for the given obligation
  | "tree"
  // Jump to the definition of the item
  | "jump-to-def"
  // Add a highlight to the current file
  | "add-highlight"
  // Remove a highlight from the current file
  | "remove-highlight";

export type PanoptesToSystemMsg<T extends PanoptesToSystemCmds> = {
  command: T;
  type: FROM_WV;
} & (T extends "jump-to-def" // Does not require the common data
  ? { location: DefLocation }
  : CommonData &
      (T extends "obligations"
        ? {}
        : T extends "tree"
          ? { predicate: Obligation; range: CharRange }
          : T extends "add-highlight" | "remove-highlight"
            ? { range: CharRange }
            : never));

// ------------------------------------------------------
// Interface between the system and rustc plugin

export type ArgusCliOptions =
  // Type-check the open workspace (eqv to running `cargo check`)
  | "preload"
  // Generate a proof tree for the given obligation
  | "tree"
  // Record obligations for the given file
  | "obligations";

/**
 * The arguments associated with each command for invoking the Argus backend.
 */
export type ArgusArgs<T extends ArgusCliOptions> = T extends "preload"
  ? ["preload"]
  : T extends "obligations"
    ? ["obligations", Filename]
    : T extends "tree"
      ? ["tree", Filename, string, number, number, number, number]
      : never;

export type ArgusReturn<T extends ArgusCliOptions> = T extends "preload"
  ? undefined
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
  _noOutput?: boolean,
  _ignoreExitCode?: boolean
) => CPromise<ArgusResult<T>>;

// Type predicates (these shouldn't really exist ...)

function objWCmd(m: any): m is { command: string } {
  return typeof m === "object" && "command" in m;
}

const makeSysMsgPredicateF =
  <T extends SystemToPanoptesCmds>(cmd: T) =>
  (msg: unknown): msg is SystemToPanoptesMsg<T> =>
    objWCmd(msg) && msg.command === cmd;

export const isSysMsgOpenError = makeSysMsgPredicateF("open-error");
export const isSysMsgOpenFile = makeSysMsgPredicateF("open-file");
export const isSysMsgHavoc = makeSysMsgPredicateF("havoc");
export const isSysMsgPin = makeSysMsgPredicateF("pin");
export const isSysMsgUnpin = makeSysMsgPredicateF("unpin");

// ------------------------------------------------------

const makePanoMsgPredicateF =
  <T extends PanoptesToSystemCmds>(cmd: T) =>
  (msg: unknown): msg is PanoptesToSystemMsg<T> =>
    objWCmd(msg) && msg.command === cmd;

export const isPanoMsgObligations = makePanoMsgPredicateF("obligations");
export const isPanoMsgTree = makePanoMsgPredicateF("tree");
export const isPanoMsgAddHighlight = makePanoMsgPredicateF("add-highlight");
export const isPanoMsgRemoveHighlight =
  makePanoMsgPredicateF("remove-highlight");
export const isPanoMsgJumpToDef = makePanoMsgPredicateF("jump-to-def");

export interface IssueOptions {
  osPlatform: string;
  osRelease: string;
  vscodeVersion: string;
  logText: string;
}

export function getArgusIssueUrl(err: string, opts: IssueOptions) {
  const bts = "```";
  const url = newGithubIssueUrl({
    user: "cognitive-engineering-lab",
    repo: "argus",
    body: `# Problem
<!-- Please describe the problem and how you encountered it. -->

# Logs
<!-- You don't need to add or change anything below this point. -->

**OS:** ${opts.osPlatform} (${opts.osRelease})
**VSCode:** ${opts.vscodeVersion}
**Error message**
${bts}
${err}
${bts}
${opts.logText}`
  });

  return url;
}
