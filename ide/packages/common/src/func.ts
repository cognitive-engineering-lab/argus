import _ from "lodash";

import type {
  BoundRegionKind,
  BoundTyKind,
  BoundVariableKind,
  CharRange,
  EvaluationResult,
  GenericArg,
  ObligationHash,
  ObligationNecessity,
  Predicate,
  Region,
  Ty,
  TyVal
} from "./bindings";
import type { MessageSystem } from "./communication";
import type { Filename } from "./lib";

export const arrUpdate = <T>(arr: T[], idx: number, val: T) =>
  _.map(arr, (v, i) => (i !== idx ? v : val));

export function isObject(x: any): x is object {
  return typeof x === "object" && x !== null;
}

export function obligationCardId(file: Filename, hash: ObligationHash) {
  const name = file.split(/[\\/]/).pop();
  return `obl--${name}-${hash}`;
}

export function bringToFront<T>(data: T[], index: number): T[] {
  return [data[index], ...data.slice(0, index), ...data.slice(index + 1)];
}

export function errorCardId(
  file: Filename,
  bodyIdx: number,
  errIdx: number,
  errType: "trait" | "ambig"
) {
  const name = file.split(/[\\/]/).pop();
  return `err--${name}-${bodyIdx}-${errType}-${errIdx}`;
}

export function makeHighlightPosters(
  messageSystem: MessageSystem,
  range: CharRange,
  file: Filename
) {
  const addHighlight = () => {
    messageSystem.postData("add-highlight", {
      type: "FROM_WEBVIEW",
      file,
      range
    });
  };

  const removeHighlight = () => {
    messageSystem.postData("remove-highlight", {
      type: "FROM_WEBVIEW",
      file,
      range
    });
  };

  return [addHighlight, removeHighlight];
}

export const isVisibleObligation = (
  o: { necessity: ObligationNecessity; result: EvaluationResult },
  filterAmbiguities = false
) =>
  // Short-circuit ambiguities if we're filtering them
  !(
    (o.result === "maybe-ambiguity" || o.result === "maybe-overflow") &&
    filterAmbiguities
  ) &&
  // If the obligation is listed as necessary, it's visible
  (o.necessity === "Yes" ||
    // If the obligation is listed as necessary on error, and it failed, it's visible
    (o.necessity === "OnError" && o.result === "no"));

export function searchObject(obj: any, target: any) {
  for (let key in obj) {
    if (obj[key] === target) {
      return true;
    }

    if (typeof obj[key] === "object" && obj[key] !== null) {
      if (searchObject(obj[key], target)) {
        return true;
      }
    }
  }

  return obj === target;
}

export function mean(arr: number[]) {
  return _.sum(arr) / arr.length;
}

export function mode(arr: number[]) {
  const counts = _.countBy(arr);
  const max = _.max(_.values(counts));
  return _.findKey(counts, v => v === max);
}

export function stdDev(arr: number[], avg: number) {
  return Math.sqrt(_.sum(_.map(arr, n => (n - avg) ** 2)) / arr.length);
}

// FIXME: take into account the column ...
export function rangeContains(outer: CharRange, inner: CharRange) {
  return (
    outer.start.line <= inner.start.line && inner.end.line <= outer.end.line
  );
}

export function anyElems(...lists: any[][]) {
  return _.some(lists, l => l.length > 0);
}

// NOTE: difference between this and _.takeRightWhile is that
// this *does* include the first element that matches the predicate.
export function takeRightUntil<T>(arr: T[], pred: (t: T) => boolean) {
  if (arr.length <= 1) {
    return arr;
  }

  let i = arr.length - 1;
  while (0 <= i) {
    if (pred(arr[i])) {
      break;
    }
    i--;
  }
  return arr.slice(i, arr.length);
}

export type Unit = { Tuple: Ty[] };

export function isUnitTy(o: TyVal): o is Unit {
  return isObject(o) && "Tuple" in o && o.Tuple.length === 0;
}

export function isTraitClause(predicate: Predicate): boolean {
  const value = predicate.value;
  if (isObject(value) && "Clause" in value) {
    const clause = value.Clause;
    if ("Trait" in clause) {
      return true;
    }
  }

  return false;
}

export function fnInputsAndOutput<T>(args: T[]): [T[], T] {
  if (args.length === 0) {
    throw new Error("fnInputsAndOutput: no arguments provided.");
  }

  // Get all elements from 0 to args.length - 1
  let inputs = _.slice(args, 0, args.length - 1);
  let output = _.last(args)!;
  return [inputs, output];
}

export const isNamedRegion = (r: Region) => r.type === "Named";

export function isNamedGenericArg(ga: GenericArg) {
  return "Lifetime" in ga ? isNamedRegion(ga.Lifetime) : true;
}

export const isNamedBoundRegion = (br: BoundRegionKind) =>
  isObject(br) && "BrNamed" in br && br.BrNamed[0] !== "'_";

export const isNamedBoundTy = (bt: BoundTyKind) =>
  isObject(bt) && "Param" in bt;

export function isNamedBoundVariable(bv: BoundVariableKind) {
  if (isObject(bv)) {
    if ("Region" in bv) {
      return isNamedBoundRegion(bv.Region);
    } else if ("Ty" in bv) {
      return isNamedBoundTy(bv.Ty);
    }
  }

  return false;
}

export function makeid(length = 16) {
  let result = "";
  const characters =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  const charactersLength = characters.length;
  let counter = 0;
  while (counter < length) {
    result += characters.charAt(Math.floor(Math.random() * charactersLength));
    counter += 1;
  }
  return result;
}
