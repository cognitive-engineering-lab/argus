import {
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

export interface TreeView {
  topology: TreeTopology;
  underlying?: TreeView;
}

type ControlFlow = "keep" | "remove-tree" | "remove-node";

export class TreeInfo {
  private view: TreeView;

  public constructor(private readonly tree: SerializedTree) {
    const childrenOf = (n: ProofNodeIdx) => {
      return tree.topology.children[n] ?? [];
    };
    const cf = (n: ProofNodeIdx): ControlFlow => {
      const node = tree.nodes[n];
      switch (node.type) {
        case "goal": {
          if (node.data.necessity.type === "yes") {
            return "keep";
          } else {
            return "remove-tree";
          }
        }
        case "candidate": {
          if (node.data.type === "any") {
            return "remove-node";
          } else {
            return "keep";
          }
        }
        default:
          return "keep";
      }
    };
    this.view = makeTreeView(tree.root, cf, childrenOf);
    console.debug("Tree abstracted view", this.view);
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

    const recommendedOrder = _.sortBy(viewLeaves, [
      leaf => {
        const node = this.tree.nodes[leaf];
        switch (node.type) {
          case "result": {
            switch (node.data) {
              case "no":
                return 0;
              case "maybe-overflow":
              case "maybe-ambiguity":
                return 1;
              case "yes":
                throw new Error("Only expected error leaves.");
            }
          }
          default:
            throw new Error("Leaves should only be results.");
        }
        // if (node.type === "result" && node.data)
        // switch (leaf.data)
      },
      leaf => {
        const pathToRoot = this.pathToRoot(leaf);
        const numInferVars = _.map(pathToRoot.path, idx => {
          const node = this.tree.nodes[idx];
          switch (node.type) {
            case "goal":
              return node.data.numVars;
            default:
              return 0;
          }
        });
        // Sort the leaves by the ration of inference variables to path length.
        return _.reduce(numInferVars, _.add, 0) / pathToRoot.path.length;
      },
    ]);

    return recommendedOrder;
  }
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

export default TreeInfo;
