export type { BodyBundle } from "@argus/common/bindings";
import { createClosedMessageSystem } from "@argus/common/communication";
import type { PanoptesConfig } from "@argus/common/lib";
import React from "react";
import App, { webSysSpec } from "./App";

export type { PanoptesConfig };

export { App as UnsafeApp };

const Panoptes = ({ config }: { config: PanoptesConfig }) => {
  if (config.type === "VSCODE_BACKING")
    throw new Error(
      "The `Panoptes` component does not support VSCode backed systems"
    );

  const spec = webSysSpec;
  const system = createClosedMessageSystem(config.closedSystem);
  config.evalMode = config.evalMode ?? "release";
  config.rankMode = config.rankMode ?? "inertia";

  const configNoUndef: Required<PanoptesConfig> = config as any;
  return <App config={configNoUndef} system={system} spec={spec} />;
};

export default Panoptes;
