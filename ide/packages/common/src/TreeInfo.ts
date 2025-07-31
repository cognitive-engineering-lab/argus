import _ from "lodash";

import type {
  CandidateData,
  CandidateIdx,
  EvaluationResult,
  GoalIdx,
  GoalKind,
  GraphTopology,
  Heuristic,
  Implementors,
  ProofNode,
  ProofNodeUnpacked,
  ResultIdx,
  SerializedTree,
  SetHeuristic
} from "./bindings";
import type { SortStrategy } from "./lib";

export type TreeViewWithRoot = TreeView & { root: ProofNode };

export interface TreeView {
  topology: GraphTopology;
  underlying?: TreeView;
}

type MultiRecord<K extends number, T> = Record<K, T[]>;

type Direction = "to-root" | "from-root";

type Reverse<T extends Direction> = T extends "to-root"
  ? "from-root"
  : "to-root";

function reverseDirection<D extends Direction>(d: Direction): Reverse<D> {
  // HACK: ugh, get rid of the `any` here.
  return d === "to-root" ? ("from-root" as any) : ("to-root" as any);
}

const BEST_EFFORT_PATH_BFS_MAX_LENGTH = 128;
const BEST_EFFORT_PATH_BFS_MAX_BREADTH = 128;

class Path<T, D extends Direction> {
  constructor(
    private readonly from: T,
    private readonly to: T,
    private readonly path: T[],
    private readonly d: D
  ) {
    if (_.first(path) !== from) {
      throw new Error("Path does not start from the `from` node");
    }

    if (_.last(path) !== to) {
      throw new Error("Path does not end at the `to` node");
    }
  }

  get pathInclusive() {
    return this.path;
  }

  get length() {
    return this.path.length;
  }

  reverse(): Path<T, Reverse<D>> {
    return new Path(
      this.to,
      this.from,
      _.reverse(this.path),
      reverseDirection(this.d)
    );
  }
}

function makeTreeView(
  root: ProofNode,
  cf: (n: ProofNode) => ControlFlow,
  childrenOf: (n: ProofNode) => ProofNode[]
): TreeView | undefined {
  const children: MultiRecord<ProofNode, ProofNode> = {};
  const parents: MultiRecord<ProofNode, ProofNode> = {};
  const addChildRel = (from: ProofNode, to: ProofNode) => {
    if (children[from]) {
      children[from].push(to);
    } else {
      children[from] = [to];
    }
    if (parents[to]) {
      parents[to].push(from);
    } else {
      parents[to] = [from];
    }
  };

  const iterate = (curr: ProofNode, prev?: ProofNode) => {
    const alreadyVisitedSomeChildOfCurr = curr in children;
    const kids = childrenOf(curr);
    let newPrev = prev;
    switch (cf(curr)) {
      case "keep": {
        if (prev !== undefined) {
          addChildRel(prev, curr);
        }
        newPrev = curr;
        if (alreadyVisitedSomeChildOfCurr) {
          // already processed; continuing vs. not would affect termination without
          // affecting the results computed
          return;
        }
        break;
      }
      case "remove-node":
        break;
      case "remove-tree":
        return;
    }
    _.forEach(kids, kid => iterate(kid, newPrev));
  };

  iterate(root);
  console.debug(`CF for root ${root} ${cf(root)}`);

  if (children[root] !== undefined) {
    return {
      topology: { children, parents }
    };
  }
}

type ControlFlow = "keep" | "remove-tree" | "remove-node";

export function invertViewWithRoots(
  leaves: ProofNode[],
  tree: TreeInfo
): TreeViewWithRoot[] {
  const invertedTopology = {
    children: tree.topology.parents,
    parents: tree.topology.children
  };
  return leaves.map(leaf => ({
    topology: invertedTopology,
    root: leaf
  }));
}

function isBadUnification(kind: GoalKind) {
  return (
    kind.type === "DeleteFnParams" ||
    kind.type === "AddFnParams" ||
    kind.type === "IncorrectParams"
  );
}

export const unpackProofNode: (node: ProofNode) => ProofNodeUnpacked = node => {
  // Extract the index by masking out the top 2 bits
  const idx = node & ((1 << 30) - 1); // u32::MAX >> 2 equivalent

  // Check the top 2 bits to determine the type
  const topBits = node & ((1 << 31) | (1 << 30));

  if (topBits === 0) {
    // Both top bits are 0 -> Goal
    return { Goal: idx };
  } else if ((node & (1 << 31)) !== 0) {
    // Top bit is 1 -> Candidate
    return { Candidate: idx };
  } else {
    // Only second bit is 1 -> Result
    return { Result: idx };
  }
};

export class TreeInfo {
  private numInferVars: Map<ProofNode, number>;

  static new(tree: SerializedTree, showHidden = false) {
    const childrenOf = (n: ProofNode) => {
      return tree.topology.children[n] ?? [];
    };
    const cf = (node: ProofNode): ControlFlow => {
      if (showHidden) {
        return "keep";
      }
      const unpacked = unpackProofNode(node);
      if ("Goal" in unpacked) {
        const goalData = tree.goals[unpacked.Goal];
        const result = tree.results[goalData.result];
        return "keep";
        // FIXME: I believe that this logic is correct, but argus crashes when enabled
        // return isHiddenObl({ necessity: goalData.necessity, result })
        //   ? "remove-tree"
        //   : "remove-node";
      }
      if ("Candidate" in unpacked) {
        const candidate = tree.candidates[unpacked.Candidate];
        return "Any" in candidate ? "remove-node" : "keep";
      }
      return "keep";
    };

    const view = makeTreeView(tree.root, cf, childrenOf);
    if (view !== undefined) {
      return new TreeInfo(tree, showHidden, view);
    }
  }

  private constructor(
    private readonly tree: SerializedTree,
    readonly showHidden: boolean,
    readonly view: TreeView
  ) {
    this.numInferVars = new Map();
  }

  get topology(): GraphTopology {
    return this.view.topology;
  }

  get root(): ProofNode {
    return this.tree.root;
  }

  public numFailedSets() {
    return this.failedSets().length;
  }

  public failedSetsSorted(sortAs: SortStrategy = "inertia"): SetHeuristic[] {
    const sets = this.failedSets();

    switch (sortAs) {
      case "inertia":
        return _.sortBy(sets, TreeInfo.setInertia);
      case "vars":
        return _.sortBy(sets, s => this.setInferVars(s));
      default:
        throw new Error("Unknown sort strategy");
    }
  }

  private failedSets(): SetHeuristic[] {
    if (this.showHidden) return this.tree.analysis.problematicSets;

    const setHasBadUnification = (s: SetHeuristic) =>
      _.some(s.goals, g => isBadUnification(g.kind));

    // Find the lowest inertia set that *does not* have a unification failure.
    const nonUnificationFailureLowestInertia = _.min(
      _.map(this.tree.analysis.problematicSets, s =>
        setHasBadUnification(s) ? undefined : TreeInfo.setInertia(s)
      )
    );

    // If all the problematic sets involve a bad unification, then we
    // have to live with them, don't filter.
    if (nonUnificationFailureLowestInertia === undefined) {
      return this.tree.analysis.problematicSets;
    }

    // Keep the sets that *don't* have unification failures OR have an
    // inertia lower than `nonUnificationFailureLowestInertia`.
    return _.filter(
      this.tree.analysis.problematicSets,
      s =>
        !setHasBadUnification(s) ||
        TreeInfo.setInertia(s) < nonUnificationFailureLowestInertia
    );
  }

  private unificationFailures(): ProofNode[] {
    const goals = _.flatMap(this.tree.analysis.problematicSets, s => s.goals);
    return _.map(
      _.filter(goals, g => isBadUnification(g.kind)),
      g => g.proofNode
    );
  }

  private nodesInUnificationFailurePath(): ProofNode[] {
    if (this.showHidden) return [];

    const nonUnificationFailures = _.flatMap(
      _.flatMap(this.failedSets(), s => _.map(s.goals, g => g.proofNode)),
      n => this.pathToRoot(n)?.pathInclusive ?? []
    );

    const uFs = _.flatMap(
      this.unificationFailures(),
      n => this.pathToRoot(n)?.pathInclusive ?? []
    );

    return _.difference(uFs, nonUnificationFailures);
  }

  public goalOfNode(n: ProofNode) {
    const node = unpackProofNode(n);
    return "Goal" in node ? this.goal(node.Goal) : undefined;
  }

  public candidate(n: CandidateIdx): CandidateData {
    return this.tree.candidates[n];
  }

  public goal(n: GoalIdx) {
    return this.tree.goals[n];
  }

  public parents(n: ProofNode): ProofNode[] | undefined {
    return this.view.topology.parents[n];
  }

  public children(n: ProofNode): ProofNode[] {
    const nodesToUnifyFailures = this.nodesInUnificationFailurePath();
    const children = this.view.topology.children[n] ?? [];
    return _.difference(children, nodesToUnifyFailures);
  }

  public result(n: ResultIdx): EvaluationResult {
    return this.tree.results[n];
  }

  public resultOfGoal(n: GoalIdx): EvaluationResult {
    return this.result(this.goal(n).result);
  }

  public nodeResult(n: ProofNode): EvaluationResult | undefined {
    const node = unpackProofNode(n);
    if ("Result" in node) {
      return this.result(node.Result);
    } else if ("Goal" in node) {
      return this.resultOfGoal(node.Goal);
    } else {
      return undefined;
    }
  }

  public pathToRoot(from: ProofNode): Path<ProofNode, "to-root"> | undefined {
    // bounded BFS
    type Entry = {
      byWayOf: ProofNode | undefined;
    };
    let shortestPaths: Map<ProofNode, Entry> = new Map();
    shortestPaths.set(from, { byWayOf: undefined });
    let frontier = shortestPaths;
    for (
      let pathLength = 0;
      pathLength < BEST_EFFORT_PATH_BFS_MAX_LENGTH;
      pathLength++
    ) {
      let nextFrontier: Map<ProofNode, Entry> = new Map();
      for (const [target, _] of frontier) {
        const parents = this.parents(target);
        if (target === this.root) {
          let pathReversed = [];
          for (
            let current: ProofNode | undefined = target;
            current !== undefined;
            current = shortestPaths.get(current)?.byWayOf
          ) {
            pathReversed.push(current);
          }
          return new Path(from, this.root, pathReversed.reverse(), "to-root");
        }
        for (const parent of parents ?? []) {
          const alreadyReached = shortestPaths.get(parent) !== undefined;
          if (!alreadyReached) {
            nextFrontier.set(parent, { byWayOf: target });
          }
        }
      }
      if (
        frontier.size === 0 ||
        frontier.size > BEST_EFFORT_PATH_BFS_MAX_BREADTH
      ) {
        return undefined;
      }
      for (const entry of nextFrontier) {
        shortestPaths.set(entry[0], entry[1]);
      }
    }
  }

  public inferVars(n: ProofNode): number {
    const current = this.numInferVars.get(n);
    if (current !== undefined) {
      return current;
    }
    const node = unpackProofNode(n);
    const niv = _.reduce(
      this.children(n),
      (sum, k) => sum + this.inferVars(k),
      "Goal" in node ? this.goal(node.Goal).numVars : 0
    );
    this.numInferVars.set(n, niv);
    return niv;
  }

  /**
   * Define the heuristic used for inertia in the system. Previously we were
   * using `momentum / velocity` but this proved too sporadic. Some proof trees
   * were deep, needlessely, and this threw a wrench in the order.
   */
  public static setInertia = (set: SetHeuristic) => {
    return set.inertia;
  };

  public setInferVars(set: SetHeuristic) {
    const heuristicVars = (h: Heuristic) => this.inferVars(h.proofNode);
    return _.sum(_.map(set.goals, heuristicVars));
  }

  public minInertiaOnPath(n: ProofNode): number {
    const hs: SetHeuristic[] = _.filter(this.failedSets(), h =>
      _.some(h.goals, g => {
        const pathToRoot = this.pathToRoot(g.proofNode);
        if (!pathToRoot) {
          return g.proofNode === n;
        }
        return _.includes(this.pathToRoot(g.proofNode)?.pathInclusive, n);
      })
    );

    // HACK: the high default is a hack to get rid of undefined,
    // but it should never be undefined.
    return _.min(_.map(hs, TreeInfo.setInertia)) ?? 10_000;
  }

  public implCandidates(node: ProofNode): Implementors | undefined {
    const unpacked = unpackProofNode(node);
    if ("Goal" in unpacked) {
      const implIdx = this.tree.impls[unpacked.Goal];
      return this.tree.implementors[implIdx];
    }
  }
}

export default TreeInfo;
