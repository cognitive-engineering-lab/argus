import { ProofNodeIdx, TreeTopology } from "@argus/common/bindings";
import _ from "lodash";
import React, { useContext } from "react";

import { TreeContext } from "./Context";
import { CollapsibleElement, DirRecursive } from "./Directory";
import { TreeInfo, TreeView } from "./TreeInfo";

type TreeViewWithRoot = TreeView & { root: ProofNodeIdx };

class TopologyBuilder {
  private topo: TreeTopology;
  constructor(readonly root: ProofNodeIdx, readonly tree: TreeInfo) {
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
    console.debug(
      "Adding path",
      _.keys(this.topo.children).length,
      _.keys(this.topo.parent).length,
      path
    );
    const thisRoot = _.head(path);
    if (
      thisRoot === undefined ||
      !_.isEqual(this.tree.node(thisRoot), this.tree.node(this.root))
    ) {
      console.error("Path does not start from the root", {
        a: thisRoot,
        b: this.root,
        c: this.tree.node(thisRoot!),
        d: this.tree.node(this.root),
      });
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
function invertViewWithRoots(
  leaves: ProofNodeIdx[],
  tree: TreeInfo
): Array<TreeViewWithRoot> {
  console.debug("Initial leaves", leaves);

  const groups: ProofNodeIdx[][] = _.values(
    _.groupBy(leaves, leaf => {
      const node = tree.node(leaf);
      if ("Goal" in node) {
        return node.Goal;
      } else {
        throw new Error("Leaves must be goals");
      }
    })
  );

  return _.map(groups, group => {
    // Each element of the group is equivalent, so just take the first
    const builder = new TopologyBuilder(group[0], tree);

    // Get the paths to the root from all leaves, filter paths that
    // contain successful nodes.
    const pathsToRoot = _.map(group, parent => tree.pathToRoot(parent).path);

    _.forEach(pathsToRoot, path => {
      // No need to take the tail, `addPathFromRoot` checks that the
      // roots are equal and then skips the first element.
      builder.addPathFromRoot(path);
    });

    return builder.toView();
  });
}

const BottomUp = () => {
  const tree = useContext(TreeContext)!;
  const mkGetChildren = (view: TreeView) => (idx: ProofNodeIdx) =>
    view.topology.children[idx] ?? [];

  const liftToGoal = (idx: ProofNodeIdx) => {
    let curr: ProofNodeIdx | undefined = idx;
    while (curr !== undefined && !("Goal" in tree.node(curr))) {
      curr = tree.parent(curr);
    }
    return curr;
  };

  const leaves = _.map(tree.errorLeaves(), liftToGoal);

  const invertedViews = invertViewWithRoots(_.compact(leaves), tree);
  // The "Argus recommended" errors are shown expanded, and the
  // "others" are collapsed. Argus recommended errors are the ones
  // that failed or are ambiguous with a concrete type on the LHS.
  const [argusRecommended, others] = _.partition(invertedViews, view => {
    const node = tree.node(view.root);
    if ("Goal" in node) {
      const goal = tree.goal(node.Goal);
      const result = tree.result(goal.result);
      return !goal.isMainTv && (result === "no" || result === "maybe-overflow");
    } else {
      // Leaves should only be goals...
      throw new Error(`Leaves should only be goals ${node}`);
    }
  });

  const LeafElement = ({ leaf }: { leaf: TreeViewWithRoot }) => (
    <DirRecursive
      level={[leaf.root]}
      getNext={mkGetChildren(leaf)}
      styleEdges={false}
    />
  );

  const recommendedSortedViews = tree.sortByRecommendedOrder(
    argusRecommended,
    v => v.root
  );
  const recommended = _.map(recommendedSortedViews, (leaf, i) => (
    <LeafElement key={i} leaf={leaf} />
  ));

  const fallbacks =
    others.length === 0 ? null : (
      <CollapsibleElement
        info={<span>Other failures ...</span>}
        Children={() => (
          <>
            {_.map(others, (leaf, i) => (
              <LeafElement key={i} leaf={leaf} />
            ))}
          </>
        )}
      />
    );

  return (
    <>
      {recommended}
      {fallbacks}
    </>
  );
};

export default BottomUp;
