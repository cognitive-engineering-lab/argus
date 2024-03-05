import { ErrorJumpTargetInfo } from "@argus/common/lib";
import _ from "lodash";
import { action, makeObservable, observable } from "mobx";

class HighlightTarget {
  value?: ErrorJumpTargetInfo;

  constructor() {
    makeObservable(this, {
      value: observable,
      reset: action,
      set: action,
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

export const highlightedObligation = new HighlightTarget();
