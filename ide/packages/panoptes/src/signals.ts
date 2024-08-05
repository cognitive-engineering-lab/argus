import type { DefinedPath, TyVal } from "@argus/common/bindings";
import type { ErrorJumpTargetInfo } from "@argus/common/lib";
import type { TypeContext } from "@argus/print/context";
import { action, makeObservable, observable } from "mobx";
import type { ReactElement } from "react";

class HighlightTarget {
  private static DURATION = 1000;

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
    console.debug("Setting highlight target", info);
    this.value = info;

    // The target value for a highlight should only last for a short time.
    window.setTimeout(() => this.reset(), HighlightTarget.DURATION);
  }

  reset() {
    this.value = undefined;
  }
}

export const HighlightTargetStore = new HighlightTarget();

// MiniBuffer data that should *not* rely on type context
type DataNoCtx = {
  kind: "argus-note";
  data: ReactElement;
};

// MiniBuffer data that *must* provide type context
type DataWithCtx = { ctx: TypeContext } & (
  | {
      kind: "path";
      path: DefinedPath;
    }
  | {
      kind: "projection";
      original: TyVal;
      projection: TyVal;
    }
);

export type BufferDataKind = { pinned?: boolean } & (DataWithCtx | DataNoCtx);

class MiniBufferData {
  data?: BufferDataKind;

  constructor() {
    makeObservable(this, {
      data: observable,
      set: action,
      reset: action,
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

  set(data: BufferDataKind) {
    // Don't override data that is pinned.
    if (this.data === undefined || !this.data.pinned) {
      this.data = { ...data, pinned: false } as BufferDataKind;
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
