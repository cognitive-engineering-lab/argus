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

  if (children[root] !== undefined) {
    return {
      topology: { children, parent },
    };
  }
}

export interface TreeView {
  topology: TreeTopology;
  underlying?: TreeView;
}

type ControlFlow = "keep" | "remove-tree" | "remove-node";

export class TreeInfo {
  private _maxHeight: Map<ProofNodeIdx, number>;
  private numInferVars: Map<ProofNodeIdx, number>;

  static new(tree: SerializedTree, showHidden: boolean = false) {
    const childrenOf = (n: ProofNodeIdx) => {
      return tree.topology.children[n] ?? [];
    };
    const cf = (n: ProofNodeIdx): ControlFlow => {
      if (showHidden) {
        return "keep";
      }

      const node = tree.nodes[n];
      if ("Goal" in node) {
        return "keep";
        // const goalData = tree.goals[node.Goal];
        // const result = tree.results[goalData.result];
        // return isHiddenObl({ necessity: goalData.necessity, result })
        //   ? "remove-tree"
        //   : "keep";
      } else if ("Candidate" in node) {
        const candidate = tree.candidates[node.Candidate];
        return "Any" in candidate ? "remove-node" : "keep";
      } else {
        return "keep";
      }
    };

    const view = makeTreeView(tree.root, cf, childrenOf);
    if (view !== undefined) {
      return new TreeInfo(tree, showHidden, view);
    }
  }

  constructor(
    private readonly tree: SerializedTree,
    readonly showHidden: boolean = false,
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

  public node(n: ProofNodeIdx) {
    return this.tree.nodes[n];
  }

  public depth(n: ProofNodeIdx) {
    return this.pathToRoot(n).path.length;
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
}

export default TreeInfo;
