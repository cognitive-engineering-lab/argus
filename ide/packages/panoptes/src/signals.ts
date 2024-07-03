import type { DefinedPath } from "@argus/common/bindings";
import type { ErrorJumpTargetInfo } from "@argus/common/lib";
import type { TypeContext } from "@argus/print/context";
import { action, makeObservable, observable } from "mobx";

class HighlightTargetStore {
  value?: ErrorJumpTargetInfo;

  constructor() {
    makeObservable(this, {
      value: observable,
      reset: action,
      set: action
    });
    this.value = undefined;
  }

  set(info: ErrorJumpTargetInfo) {
    this.value = info;
  }

  reset() {
    this.value = undefined;
  }
}

export const highlightedObligation = new HighlightTargetStore();

export type BufferDataKind = {
  ctx: TypeContext;
  pinned: boolean;
} & {
  kind: "path";
  path: DefinedPath;
};

class MiniBufferData {
  data?: BufferDataKind;

  constructor() {
    makeObservable(this, {
      data: observable,
      set: action,
      pin: action,
      unpin: action
    });
    this.data = undefined;
  }

  unpin() {
    if (this.data !== undefined) {
      this.data.pinned = false;
    }
  }

  pin() {
    if (this.data !== undefined) {
      this.data.pinned = true;
    }
  }

  set(data: Omit<BufferDataKind, "pinned">) {
    // Don't override data that is pinned.
    if (this.data === undefined || !this.data.pinned) {
      this.data = { pinned: false, ...data };
    }
  }

  reset() {
    // Don't clear data that is pinned
    if (this.data !== undefined && !this.data.pinned) {
      this.data = undefined;
    }
  }
}

export const MiniBufferDataStore = new MiniBufferData();
