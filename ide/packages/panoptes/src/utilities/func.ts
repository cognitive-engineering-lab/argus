import {
  CharRange,
  Obligation,
  ObligationHash,
  Predicate,
  Ty,
} from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";
import _ from "lodash";

import { MessageSystem } from "../communication";

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
    messageSystem.postData({
      type: "FROM_WEBVIEW",
      file,
      command: "add-highlight",
      range,
    });
  };

  const removeHighlight = () => {
    messageSystem.postData({
      type: "FROM_WEBVIEW",
      file,
      command: "remove-highlight",
      range,
    });
  };

  return [addHighlight, removeHighlight];
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

export function fnInputsAndOutput<T>(args: T[]): [T[], T] {
  if (args.length === 0) {
    throw new Error("fnInputsAndOutput: no arguments provided.");
  }

  // Get all elements from 0 to args.length - 1
  let inputs = _.slice(args, 0, args.length - 1);
  let output = _.last(args)!;
  return [inputs, output];
}

export type Unit = { Tuple: Ty[] };

export function tyIsUnit(o: Ty): o is Unit {
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

export function isHiddenObl(o: { necessity: string; result: string }) {
  return (
    o.necessity === "Yes" || (o.necessity === "OnError" && o.result === "no")
  );
}

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
  return Math.sqrt(_.sum(_.map(arr, n => Math.pow(n - avg, 2))) / arr.length);
}
