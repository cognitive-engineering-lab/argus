import { makeAutoObservable } from "mobx";
import { createContext } from "react";

import { SerializedTree } from "@argus/common/bindings";

export class ActiveState {
  currentNode: number | null = null;
  constructor() {
    makeAutoObservable(this);
  }

  setActiveNode(node: number) {
    this.currentNode = node;
  }

  getActiveNode() {
    return this.currentNode;
  }
}

export const ActiveContext = createContext<ActiveState | null>(null);
export const TreeContext = createContext<SerializedTree | null>(null);