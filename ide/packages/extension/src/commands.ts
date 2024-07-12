import type { BodyHash, ExprIdx, ObligationHash } from "@argus/common/bindings";
import type { Filename } from "@argus/common/lib";

import type { Cmd, Ctx } from "./ctx";
import * as errors from "./errors";
import { log } from "./logging";

function trace(...args: any[]) {
  log("[CMD]: ", ...args);
}

export function inspect(ctx: Ctx): Cmd {
  return async () => {
    trace("inspect");
    ctx.inspectAt();
  };
}

export function pinMBData(ctx: Ctx): Cmd {
  return async () => {
    trace("pinMBData");
    ctx.pinMBData();
  };
}

export function unpinMBData(ctx: Ctx): Cmd {
  return async () => {
    trace("unpinMBData");
    ctx.unpinMBData();
  };
}

export function cancelTasks(ctx: Ctx): Cmd {
  return async () => {
    trace("cancelTasks");
    ctx.cancelRunningTasks();
  };
}

export function openError(ctx: Ctx): Cmd {
  return async (
    file: Filename,
    bh: BodyHash,
    ei: ExprIdx,
    oblHash: ObligationHash
  ) => {
    trace("openError", file, bh, ei, oblHash);
    ctx.openError({
      file,
      bodyIdx: bh,
      exprIdx: ei,
      hash: oblHash
    });
  };
}

export function lastError(ctx: Ctx): Cmd {
  return async () => {
    trace("lastError");
    return errors.lastError(ctx.ictxt.extCtx);
  };
}
