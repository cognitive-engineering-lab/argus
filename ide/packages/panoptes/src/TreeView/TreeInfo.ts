import {
  CandidateData,
  CandidateIdx,
  EvaluationResult,
  GoalIdx,
  ProofNodeIdx,
  ResultIdx,
  SerializedTree,
  TreeTopology,
} from "@argus/common/bindings";
import _ from "lodash";

import { isTraitClause } from "../utilities/func";

type MultiRecord<K extends number, T> = Record<K, T[]>;

type Direction = "to-root" | "from-root";

interface Path<T, D extends Direction> {
  from: T;
  to: T;
  path: T[];
  d: D;
}

function makeTreeView(
  root: ProofNodeIdx,
  cf: (n: ProofNodeIdx) => ControlFlow,
  childrenOf: (n: ProofNodeIdx) => ProofNodeIdx[]
): TreeView {
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

  if (children[root] === undefined) {
    throw new Error("Root has no children");
  }

  return {
    topology: { children, parent },
  };
}

export interface TreeView {
  topology: TreeTopology;
  underlying?: TreeView;
}

type ControlFlow = "keep" | "remove-tree" | "remove-node";

export class TreeInfo {
  private view: TreeView;
  private maxHeight: Map<ProofNodeIdx, number>;
  private numInferVars: Map<ProofNodeIdx, number>;

  public constructor(
    private readonly tree: SerializedTree,
    readonly showHidden: boolean = false
  ) {
    const childrenOf = (n: ProofNodeIdx) => {
      return tree.topology.children[n] ?? [];
    };
    const cf = (n: ProofNodeIdx): ControlFlow => {
      if (this.showHidden) {
        return "keep";
      }

      const node = tree.nodes[n];
      if ("Goal" in node) {
        if (tree.goals[node.Goal].necessity === "Yes") {
          return "keep";
        } else {
          return "remove-tree";
        }
      } else if ("Candidate" in node) {
        const candidate = this.candidate(node.Candidate);
        if ("Any" in candidate) {
          return "remove-node";
        } else {
          return "keep";
        }
      } else {
        return "keep";
      }
    };

    this.view = makeTreeView(tree.root, cf, childrenOf);
    this.maxHeight = new Map();
    this.numInferVars = new Map();
  }

  get topology(): TreeTopology {
    return this.view.topology;
  }

  get root(): ProofNodeIdx {
    return this.tree.root;
  }

  public node(n: ProofNodeIdx) {
    return this.tree.nodes[n];
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
    return this.view.topology.children[n] ?? [];
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

    return {
      from,
      to: this.root,
      path,
      d: "to-root",
    };
  }

  public pathFromRoot(from: ProofNodeIdx): Path<ProofNodeIdx, "from-root"> {
    let { from: f, to, path } = this.pathToRoot(from);
    return {
      from: to,
      to: f,
      path: path.reverse(),
      d: "from-root",
    };
  }

  public errorLeaves(): ProofNodeIdx[] {
    if (this.nodeResult(this.root) === "yes") {
      return [];
    }

    let errorLeaves = [];
    let stack = [this.root];
    while (stack.length > 0) {
      const current = stack.pop()!;
      const children = this.children(current);
      if (children.length === 0 && this.nodeResult(current) !== "yes") {
        const node = this.node(current);
        if ("Result" in node) {
          errorLeaves.push(current);
        } else {
          console.error("Node has no children but isn't a leaf", node);
        }
      } else {
        const errorKids = _.filter(children, n => this.nodeResult(n) !== "yes");
        stack.push(...errorKids);
      }
    }
    return errorLeaves;
  }

  public sortByRecommendedOrder<T>(data: T[], f: (t: T) => ProofNodeIdx): T[] {
    const sortErrorsFirst = (t: T) => {
      const leaf = f(t);
      switch (this.nodeResult(leaf)) {
        case "no":
          return 0;
        case "maybe-overflow":
        case "maybe-ambiguity":
          return 1;
        case "yes":
          return 2;
      }
    };

    const sortWeightPaths = (t: T) => {
      const leaf = f(t);
      const pathToRoot = this.pathToRoot(leaf);
      const len = pathToRoot.path.length;
      return -len;
    };

    const bubbleTraitClauses = (t: T) => {
      const leaf = f(t);
      const n = this.node(leaf);
      if ("Goal" in n && isTraitClause(this.goal(n.Goal).value.predicate)) {
        return 0;
      }
      return 1;
    };

    const recommendedOrder = _.sortBy(data, [
      bubbleTraitClauses,
      sortErrorsFirst,
      sortWeightPaths,
    ]);

    return recommendedOrder;
  }

  public errorLeavesRecommendedOrder(): ProofNodeIdx[] {
    const viewLeaves = this.errorLeaves();
    return this.sortByRecommendedOrder(viewLeaves, _.identity);
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

  public maxHeigh(n: ProofNodeIdx): number {
    const current = this.maxHeight.get(n);
    if (current !== undefined) {
      return current;
    }
    const childHeights = _.map(this.children(n), k => this.maxHeigh(k));
    const height = 1 + (_.max(childHeights) ?? 0);
    this.maxHeight.set(n, height);
    return height;
  }
}

export default TreeInfo;
