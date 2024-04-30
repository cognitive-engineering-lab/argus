import { EvaluationMode, PanoptesConfig, SystemSpec } from "@argus/common/lib";
import { createContext } from "react";

import { TreeRenderParams } from "../TreeView/Directory";
import TreeInfo from "../TreeView/TreeInfo";
import { MessageSystem } from "../communication";

export const AppContext = {
  MessageSystemContext: createContext<MessageSystem | undefined>(undefined),
  ConfigurationContext: createContext<
    (PanoptesConfig & { evalMode: EvaluationMode }) | undefined
  >(undefined),
  SystemSpecContext: createContext<SystemSpec | undefined>(undefined),
};

export const TreeAppContext = {
  TreeContext: createContext<TreeInfo | undefined>(undefined),
  TreeRenderContext: createContext<TreeRenderParams>({}),
};
