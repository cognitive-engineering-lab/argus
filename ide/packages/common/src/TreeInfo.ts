import _ from "lodash";

import type {
  CandidateData,
  CandidateIdx,
  EvaluationResult,
  GoalIdx,
  GoalKind,
  Implementors,
  ProofNodeIdx,
  ResultIdx,
  SerializedTree,
  SetHeuristic,
  TreeTopology
} from "./bindings";

export type TreeViewWithRoot = TreeView & { root: ProofNodeIdx };

export interface TreeView {
  topology: TreeTopology;
  underlying?: TreeView;
}

type MultiRecord<K extends number, T> = Record<K, T[]>;

type Direction = "to-root" | "from-root";

type Reverse<T extends Direction> = T extends "to-root"
  ? "from-root"
  : "to-root";

function reverseDirection<D extends Direction>(d: Direction): Reverse<D> {
  // HACK: ugh, get rid of the `any` here.
  return d === "to-root" ? "from-root" : ("to-root" as any);
}

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
  root: ProofNodeIdx,
  cf: (n: ProofNodeIdx) => ControlFlow,
  childrenOf: (n: ProofNodeIdx) => ProofNodeIdx[]
): TreeView | undefined {
  const children: MultiRecord<ProofNodeIdx, ProofNodeIdx> = {};
  const parent: Record<ProofNodeIdx, ProofNodeIdx> = {};
  const addChildRel = (from: ProofNodeIdx, to: ProofNodeIdx) => {
    if (children[from]) {
      children[from].push(to);
    } else {
      children[from] = [to];
    }
    if (parent[to]) {
      throw new Error("parent already set");
    }
    parent[to] = from;
  };

  const iterate = (curr: ProofNodeIdx, prev?: ProofNodeIdx) => {
    const kids = childrenOf(curr);
    let newPrev = prev;
    switch (cf(curr)) {
      case "keep": {
        if (prev !== undefined) {
          addChildRel(prev, curr);
        }
        newPrev = curr;
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
      topology: { children, parent }
    };
  }
}

type ControlFlow = "keep" | "remove-tree" | "remove-node";

class TopologyBuilder {
  private topo: TreeTopology;
  constructor(
    readonly root: ProofNodeIdx,
    readonly tree: TreeInfo
  ) {
    this.topo = { children: {}, parent: {} };
  }

  public toView(): TreeViewWithRoot {
    return { topology: this.topo, root: this.root };
  }

  get topology() {
    return this.topo;
  }

  add(from: ProofNodeIdx, to: ProofNodeIdx) {
    if (this.topo.children[from] === undefined) {
      this.topo.children[from] = [];
    }
    this.topo.children[from].push(to);
    this.topo.parent[to] = from;
  }

  /**
   *
   * @param root the root node from where this path should start.
   * @param path a path to be added uniquely to the tree.
   */
  public addPathFromRoot(path: ProofNodeIdx[]) {
    const thisRoot = _.head(path);
    if (
      thisRoot === undefined ||
      !_.isEqual(this.tree.node(thisRoot), this.tree.node(this.root))
    ) {
      throw new Error("Path does not start from the root");
    }

    let previous = this.root;
    _.forEach(_.tail(path), node => {
      // We want to add a node from `previous` to `node` only if an
      // equivalent connection does not already exist. Equivalent is
      // defined by the `Node` the `ProofNodeIdx` points to.
      const currKids = this.topo.children[previous] ?? [];
      const myNode = this.tree.node(node);
      const hasEquivalent = _.find(
        currKids,
        kid => this.tree.node(kid) === myNode
      );
      if (hasEquivalent === undefined) {
        this.add(previous, node);
        previous = node;
      } else {
        previous = hasEquivalent;
      }
    });
  }
}

/**
 * Invert the current `TreeView` on the `TreeInfo`, using `leaves` as the roots.
 * For the purpose of inverting a tree anchored at failed goals, some of these goals will
 * be 'distinct' nodes, but their inner `GoalIdx` will be the same. We want to root all of
 * these together.
 */
export function invertViewWithRoots(
  leaves: ProofNodeIdx[],
  tree: TreeInfo
): TreeViewWithRoot[] {
  const groups: ProofNodeIdx[][] = _.values(
    _.groupBy(leaves, leaf => {
      const node = tree.node(leaf);
      if ("Goal" in node) {
        return node.Goal;
      }
      throw new Error("Leaves must be goals");
    })
  );

  return _.map(groups, group => {
    // Each element of the group is equivalent, so just take the first
    const builder = new TopologyBuilder(group[0], tree);

    // Get the paths to the root from all leaves, filter paths that
    // contain successful nodes.
    const pathsToRoot = _.map(
      group,
      parent => tree.pathToRoot(parent).pathInclusive
    );

    _.forEach(pathsToRoot, path => {
      // No need to take the tail, `addPathFromRoot` checks that the
      // roots are equal and then skips the first element.
      builder.addPathFromRoot(path);
    });

    return builder.toView();
  });
}

function isBadUnification(kind: GoalKind) {
  return (
    kind.type === "DeleteFnParams" ||
    kind.type === "AddFnParams" ||
    kind.type === "IncorrectParams"
  );
}

export class TreeInfo {
  private _maxHeight: Map<ProofNodeIdx, number>;
  private numInferVars: Map<ProofNodeIdx, number>;

  static new(tree: SerializedTree, showHidden = false) {
    const childrenOf = (n: ProofNodeIdx) => {
      return tree.topology.children[n] ?? [];
    };
    const cf = (n: ProofNodeIdx): ControlFlow => {
      if (showHidden) {
        return "keep";
      }

      const node = tree.nodes[n];
      if ("Goal" in node) {
        const goalData = tree.goals[node.Goal];
        const result = tree.results[goalData.result];
        return "keep";
        // FIXME: I believe that this logic is correct, but argus crashes when enabled
        // return isHiddenObl({ necessity: goalData.necessity, result })
        //   ? "remove-tree"
        //   : "remove-node";
      }
      if ("Candidate" in node) {
        const candidate = tree.candidates[node.Candidate];
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
    this._maxHeight = new Map();
    this.numInferVars = new Map();
  }

  get topology(): TreeTopology {
    return this.view.topology;
  }

  get root(): ProofNodeIdx {
    return this.tree.root;
  }

  public failedSets() {
    if (this.showHidden) return this.tree.analysis.problematicSets;

    // If all the problematic sets involve a bad unification, then we
    // have to live with them, don't filter.
    if (
      _.every(this.tree.analysis.problematicSets, s =>
        _.some(s.goals, g => isBadUnification(g.kind))
      )
    )
      return this.tree.analysis.problematicSets;

    // Keep only the sets that don't have a bad unification
    return _.filter(this.tree.analysis.problematicSets, s =>
      _.every(s.goals, g => !isBadUnification(g.kind))
    );
  }

  private unificationFailures(): ProofNodeIdx[] {
    const goals = _.flatMap(this.tree.analysis.problematicSets, s => s.goals);
    return _.map(
      _.filter(goals, g => isBadUnification(g.kind)),
      g => g.idx
    );
  }

  private nodesInUnificationFailurePath(): ProofNodeIdx[] {
    if (this.showHidden) return [];

    const nonUnificationFailures = _.flatMap(
      _.flatMap(this.failedSets(), s => _.map(s.goals, g => g.idx)),
      n => this.pathToRoot(n).pathInclusive
    );

    const uFs = _.flatMap(
      this.unificationFailures(),
      n => this.pathToRoot(n).pathInclusive
    );

    return _.difference(uFs, nonUnificationFailures);
  }

  public node(n: ProofNodeIdx) {
    return this.tree.nodes[n];
  }

  public depth(n: ProofNodeIdx) {
    return this.pathToRoot(n).length;
  }

  public goalOfNode(n: ProofNodeIdx) {
    const node = this.node(n);
    return "Goal" in node ? this.goal(node.Goal) : undefined;
  }

  public candidate(n: CandidateIdx): CandidateData {
    return this.tree.candidates[n];
  }

  public goal(n: GoalIdx) {
    return this.tree.goals[n];
  }

  public parent(n: ProofNodeIdx): ProofNodeIdx | undefined {
    return this.view.topology.parent[n];
  }

  public children(n: ProofNodeIdx): ProofNodeIdx[] {
    const nodesToUnifyFailures = this.nodesInUnificationFailurePath();

    // if (_.includes(nodesToUnifyFailures, 6222)) {
    //   throw new Error("NODE NOT THERE");
    // } else {
    //   console.debug("Nodes to unify failures includes 6222");
    // }

    const children = this.view.topology.children[n] ?? [];
    return _.difference(children, nodesToUnifyFailures);
  }

  public result(n: ResultIdx): EvaluationResult {
    return this.tree.results[n];
  }

  public resultOfGoal(n: GoalIdx): EvaluationResult {
    return this.result(this.goal(n).result);
  }

  public nodeResult(n: ProofNodeIdx): EvaluationResult | undefined {
    const node = this.node(n);
    if ("Result" in node) {
      return this.result(node.Result);
    } else if ("Goal" in node) {
      return this.resultOfGoal(node.Goal);
    } else {
      return undefined;
    }
  }

  public pathToRoot(from: ProofNodeIdx): Path<ProofNodeIdx, "to-root"> {
    const path = [from];
    let current = from;
    while (current !== this.root) {
      const parent = this.parent(current);
      if (parent === undefined) {
        break;
      }
      path.push(parent);
      current = parent;
    }

    return new Path(from, this.root, path, "to-root");
  }

  public pathFromRoot(from: ProofNodeIdx): Path<ProofNodeIdx, "from-root"> {
    return this.pathToRoot(from).reverse();
  }

  public inferVars(n: ProofNodeIdx): number {
    const current = this.numInferVars.get(n);
    if (current !== undefined) {
      return current;
    }
    const node = this.tree.nodes[n];
    const niv = _.reduce(
      this.children(n),
      (sum, k) => sum + this.inferVars(k),
      "Goal" in node ? this.goal(node.Goal).numVars : 0
    );
    this.numInferVars.set(n, niv);
    return niv;
  }

  public maxHeight(n: ProofNodeIdx): number {
    const current = this._maxHeight.get(n);
    if (current !== undefined) {
      return current;
    }
    const childHeights = _.map(this.children(n), k => this.maxHeight(k));
    const height = 1 + (_.max(childHeights) ?? 0);
    this._maxHeight.set(n, height);
    return height;
  }

  /**
   * Define the heuristic used for inertia in the system. Previously we were
   * using `momentum / velocity` but this proved too sporadic. Some proof trees
   * were deep, needlessely, and this threw a wrench in the order.
   */
  public static setInertia = (set: SetHeuristic) => {
    return set.momentum;
  };

  public minInertiaOnPath(n: ProofNodeIdx): number {
    const hs = _.filter(this.failedSets(), h =>
      _.some(h.goals, g => _.includes(this.pathToRoot(g.idx).pathInclusive, n))
    );

    // HACK: the high default is a hack to get rid of undefined,
    // but it should never be undefined.
    return _.min(_.map(hs, TreeInfo.setInertia)) ?? 10_000;
  }

  public implCandidates(idx: ProofNodeIdx): Implementors | undefined {
    return this.tree.allImplCandidates[idx];
  }
}

export default TreeInfo;
