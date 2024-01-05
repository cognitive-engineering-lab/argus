import { Node, SerializedTree, TreeTopology } from "@argus/common/types";
import React from "react";

import { PrintGoalPredicate } from "../Ty/private/predicate";

interface Path<T, Direction> {
  from: T;
  to: T;
  path: T[];
}

type Direction = "ToRoot" | "FromRoot";

export function pathToRoot(
  tree: SerializedTree,
  from: number
): Path<number, "ToRoot"> {
  let root = tree.root;
  let topo = tree.topology;
  let path = [from];
  let current = from;
  while (current !== root) {
    let parent = topo.parent[current];
    path.push(parent);
    current = parent;
  }

  return {
    from,
    to: root,
    path,
  };
}

export function pathFromRoot(
  tree: SerializedTree,
  from: number
): Path<number, "FromRoot"> {
  let { from: f, to, path } = pathToRoot(tree, from);
  return {
    from: to,
    to: f,
    path: path.reverse(),
  };
}
