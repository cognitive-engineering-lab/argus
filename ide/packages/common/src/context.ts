import { createContext } from "react";

import type BodyInfo from "./BodyInfo";
import type TreeInfo from "./TreeInfo";
import type { MessageSystem, TreeRenderParams } from "./communication";
import type {
  EvaluationMode,
  Filename,
  PanoptesConfig,
  SystemSpec
} from "./lib";

export const AppContext = {
  MessageSystemContext: createContext<MessageSystem | undefined>(undefined),
  ConfigurationContext: createContext<
    (PanoptesConfig & { evalMode: EvaluationMode }) | undefined
  >(undefined),
  SystemSpecContext: createContext<SystemSpec | undefined>(undefined),
  ShowHiddenObligationsContext: createContext<boolean>(false)
};

export const FileContext = createContext<Filename | undefined>(undefined);

export const BodyInfoContext = createContext<BodyInfo | undefined>(undefined);

export const TreeAppContext = {
  TreeContext: createContext<TreeInfo | undefined>(undefined),
  TreeRenderContext: createContext<TreeRenderParams>({})
};
