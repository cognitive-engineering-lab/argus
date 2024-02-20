import {
  EvaluationResult,
  ProofNodeIdx,
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
        if (node.Goal.data.necessity === "Yes") {
          return "keep";
        } else {
          return "remove-tree";
        }
      } else if ("Candidate" in node) {
        if ("Any" in node.Candidate.data) {
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

  public parent(n: ProofNodeIdx): ProofNodeIdx | undefined {
    return this.view.topology.parent[n];
  }

  public children(n: ProofNodeIdx): ProofNodeIdx[] {
    return this.view.topology.children[n] ?? [];
  }

  public result(n: ProofNodeIdx): EvaluationResult | undefined {
    const node = this.node(n);
    if ("Result" in node) {
      return node.Result.data;
    } else if ("Goal" in node) {
      return node.Goal.data.result;
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

  public errorNodes(): ProofNodeIdx[] {
    const allLeaves = this.tree.errorLeaves;
    const viewLeaves = _.filter(
      allLeaves,
      leaf => this.view.topology.parent[leaf] !== undefined
    );
    return viewLeaves;
  }

  public errorNodesRecommendedOrder(): ProofNodeIdx[] {
    const viewLeaves = this.errorNodes();

    const sortErrorsFirst = (leaf: ProofNodeIdx) => {
      const node = this.tree.nodes[leaf];
      if ("Result" in node) {
        switch (node.Result.data) {
          case "no":
            return 0;
          case "maybe-overflow":
          case "maybe-ambiguity":
            return 1;
          case "yes":
            throw new Error("Only expected error leaves.");
        }
      } else {
        throw new Error("Leaves should only be results.");
      }
    };

    const sortWeightPaths = (leaf: ProofNodeIdx) => {
      const pathToRoot = this.pathToRoot(leaf);
      const numInferVars = _.map(pathToRoot.path, idx => {
        const node = this.tree.nodes[idx];
        if ("Goal" in node) {
          return node.Goal.data.numVars;
        } else {
          return 0;
        }
      });
      // Sort the leaves by the ration of inference variables to path length.
      return _.reduce(numInferVars, _.add, 0) / pathToRoot.path.length;
    };

    const recommendedOrder = _.sortBy(viewLeaves, [
      sortErrorsFirst,
      sortWeightPaths,
    ]);

    return recommendedOrder;
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
      "Goal" in node ? node.Goal.data.numVars : 0
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
