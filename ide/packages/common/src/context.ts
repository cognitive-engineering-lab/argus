import { createContext } from "react";

import type BodyInfo from "./BodyInfo";
import type TreeInfo from "./TreeInfo";
import type { MessageSystem, TreeRenderParams } from "./communication";
import type { Filename, PanoptesConfig, SystemSpec } from "./lib";

export const settingsToggles = ["show-hidden-obligations"] as const;

export type Settings = {
  [K in (typeof settingsToggles)[number]]: boolean;
};

export const AppContext = {
  MessageSystemContext: createContext<MessageSystem | undefined>(undefined),
  ConfigurationContext: createContext<Required<PanoptesConfig> | undefined>(
    undefined
  ),
  SystemSpecContext: createContext<SystemSpec | undefined>(undefined),
  SettingsContext: createContext<Settings>({ "show-hidden-obligations": false })
};

export const FileContext = createContext<Filename | undefined>(undefined);

export const BodyInfoContext = createContext<BodyInfo | undefined>(undefined);

export const TreeAppContext = {
  TreeContext: createContext<TreeInfo | undefined>(undefined),
  TreeRenderContext: createContext<TreeRenderParams>({})
};
