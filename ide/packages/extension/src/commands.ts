import { BodyHash, ExprIdx, ObligationHash } from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";

import { Cmd, Ctx } from "./ctx";
import * as errors from "./errors";

export function inspect(ctx: Ctx): Cmd {
  return async () => {
    ctx.createOrShowView();
  };
}

export function openError(ctx: Ctx): Cmd {
  return async (
    file: Filename,
    bh: BodyHash,
    ei: ExprIdx,
    oblHash: ObligationHash
  ) => {
    if (ctx.view === undefined) {
      await ctx.createOrShowView({
        file,
        bodyIdx: bh,
        exprIdx: ei,
        hash: oblHash,
      });
    }
    ctx.view!.blingObligation(file, bh, ei, oblHash);
  };
}

export function lastError(ctx: Ctx): Cmd {
  return async () => {
    return errors.lastError(ctx.extCtx);
  };
}
