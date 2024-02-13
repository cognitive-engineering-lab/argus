import { CharRange, ObligationHash } from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";
import _ from "lodash";

import { postToExtension } from "./vscode";

export function isObject(x: any): x is object {
  return typeof x === "object" && x !== null;
}

export function obligationCardId(file: Filename, hash: ObligationHash) {
  const name = file.split(/[\\/]/).pop();
  return `obl--${name}-${hash}`;
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

export function makeHighlightPosters(range: CharRange, file: Filename) {
  const addHighlight = () => {
    postToExtension({
      type: "FROM_WEBVIEW",
      file,
      command: "add-highlight",
      range,
    });
  };

  const removeHighlight = () => {
    postToExtension({
      type: "FROM_WEBVIEW",
      file,
      command: "remove-highlight",
      range,
    });
  };

  return [addHighlight, removeHighlight];
}

export function anyElems<T>(...lists: T[][]) {
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

export function fnInputsAndOutput<T>(args: [T, ...T[]]): [T[], T] {
  // Get all elements from 0 to args.length - 1
  let inputs = _.slice(args, 0, args.length - 1);
  let output = _.last(args)!;
  return [inputs, output];
}

// TODO: put these in a typed file about the rustc_middle types (when possible).

export type Unit = { Tuple: any[] };

export function tyIsUnit(o: any): o is Unit {
  return "Tuple" in o && o.Tuple.length === 0;
}