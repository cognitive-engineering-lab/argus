import { createContext } from "react";

import TreeInfo from "./TreeInfo";

export const TreeContext = createContext<TreeInfo | null>(null);
