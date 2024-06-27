import type { CancelablePromise as CPromise } from "cancelable-promise";
import newGithubIssueUrl from "new-github-issue-url";

import type {
  BodyBundle,
  BodyHash,
  CharRange,
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
  target?: ErrorJumpTargetInfo;
  evalMode?: EvaluationMode;
};

export type SystemSpec = Omit<IssueOptions, "logText">;
export type EvaluationMode = "release" | "rank" | "random";
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
  | "havoc"
  | "open-file"
  | "open-error"
  | "tree";

export type SystemToPanoptesMsg<T extends SystemToPanoptesCmds> = {
  command: T;
  type: FROM_EXT;
} & (T extends "havoc"
  ? {}
  : CommonData &
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

export function isSysMsgOpenError(
  msg: unknown
): msg is SystemToPanoptesMsg<"open-error"> {
  return objWCmd(msg) && msg.command === "open-error";
}

export function isSysMsgOpenFile(
  msg: unknown
): msg is SystemToPanoptesMsg<"open-file"> {
  return objWCmd(msg) && msg.command === "open-file";
}

export function isSysMsgHavoc(
  msg: unknown
): msg is SystemToPanoptesMsg<"havoc"> {
  return objWCmd(msg) && msg.command === "havoc";
}

// ------------------------------------------------------

export function isPanoMsgObligations(
  msg: unknown
): msg is PanoptesToSystemMsg<"obligations"> {
  return objWCmd(msg) && msg.command === "obligations";
}

export function isPanoMsgTree(
  msg: unknown
): msg is PanoptesToSystemMsg<"tree"> {
  return objWCmd(msg) && msg.command === "tree";
}

export function isPanoMsgAddHighlight(
  msg: unknown
): msg is PanoptesToSystemMsg<"add-highlight"> {
  return objWCmd(msg) && msg.command === "add-highlight";
}

export function isPanoMsgRemoveHighlight(
  msg: unknown
): msg is PanoptesToSystemMsg<"remove-highlight"> {
  return objWCmd(msg) && msg.command === "remove-highlight";
}

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
