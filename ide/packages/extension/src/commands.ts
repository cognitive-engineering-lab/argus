import { ObligationHash } from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";

import { Cmd, Ctx } from "./ctx";
import { ViewLoader } from "./view";

export function launchArgus(ctx: Ctx): Cmd {
  return async () => {
    ViewLoader.createOrShow(ctx.extCtx.extensionUri);
  };
}

export function blingObligation(_: Ctx): Cmd {
  return async (file: Filename, oblHash: ObligationHash) => {
    ViewLoader.blingObligation(file, oblHash);
  };
}

export function openError(_: Ctx): Cmd {
  return async (
    file: Filename,
    type: "ambig" | "trait",
    bodyIdx: number,
    errIdx: number
  ) => {
    ViewLoader.openError(file, type, bodyIdx, errIdx);
  };
}
