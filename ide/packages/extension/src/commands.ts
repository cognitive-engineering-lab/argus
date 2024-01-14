import { Filename } from "@argus/common/lib";
import { ObligationHash } from "@argus/common/bindings";
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
