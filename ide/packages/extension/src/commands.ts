import { BodyHash, ExprIdx, ObligationHash } from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";

import { Cmd, Ctx } from "./ctx";

export function launchArgus(ctx: Ctx): Cmd {
  return async () => {
    ctx.createOrShowView();
  };
}

export function blingObligation(ctx: Ctx): Cmd {
  return async (
    file: Filename,
    bh: BodyHash,
    ei: ExprIdx,
    oblHash: ObligationHash
  ) => {
    ctx.view!.blingObligation(file, bh, ei, oblHash);
  };
}

export function openError(ctx: Ctx): Cmd {
  return async (
    file: Filename,
    type: "ambig" | "trait",
    bodyIdx: number,
    errIdx: number
  ) => {
    ctx.view!.openError(file, type, bodyIdx, errIdx);
  };
}
