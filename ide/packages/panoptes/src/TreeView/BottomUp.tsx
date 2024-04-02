import { ProofNodeIdx, TreeTopology } from "@argus/common/bindings";
import _ from "lodash";
import React, { useContext } from "react";

import { TreeAppContext } from "../utilities/context";
import { mean, mode, searchObject, stdDev } from "../utilities/func";
import {
  CollapsibleElement,
  DirRecursive,
  TreeRenderParams,
} from "./Directory";
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
function invertViewWithRoots(
  leaves: ProofNodeIdx[],
  tree: TreeInfo
): Array<TreeViewWithRoot> {
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
  const tree = useContext(TreeAppContext.TreeContext)!;
  const mkGetChildren = (view: TreeView) => (idx: ProofNodeIdx) =>
    view.topology.children[idx] ?? [];

  const liftTo = (idx: ProofNodeIdx, target: "Goal" | "Candidate") => {
    let curr: ProofNodeIdx | undefined = idx;
    while (curr !== undefined && !(target in tree.node(curr))) {
      curr = tree.parent(curr);
    }
    return curr;
  };

  const leaves = _.uniq(
    _.compact(_.map(tree.errorLeaves(), n => liftTo(n, "Goal")))
  );
  const failedGroups = _.groupBy(leaves, leaf => tree.parent(leaf));

  // Operations on groups of errors
  const numMainUninferred = (group: ProofNodeIdx[]) =>
    _.reduce(
      group,
      (acc, leaf) =>
        acc +
        (() => {
          const node = tree.node(leaf);
          if ("Goal" in node) {
            const goal = tree.goal(node.Goal);
            return goal.isMainTv ? 1 : 0;
          } else {
            return 0;
          }
        })(),
      0
    );

  const getNumPrincipaled = (group: ProofNodeIdx[]) =>
    group.length - numMainUninferred(group);

  const sortByNPT = (group: ProofNodeIdx[]) => -getNumPrincipaled(group);

  const sortByNPTRatio = (group: ProofNodeIdx[]) =>
    group.length / getNumPrincipaled(group);

  const sortByDepth = (group: ProofNodeIdx[]) =>
    -_.max(_.map(group, leaf => tree.depth(leaf)))!;

  const takeN = (n: number) => (groups: ProofNodeIdx[][]) =>
    [_.take(groups, n), _.tail(groups)] as [ProofNodeIdx[][], ProofNodeIdx[][]];

  const takeAll = (groups: ProofNodeIdx[][]) =>
    [groups, []] as [ProofNodeIdx[][], ProofNodeIdx[][]];

  // HACK: this crappy heuristic needs to be replaced with a proper analysis.
  const [sortStrategy, firstFilter] = (() => {
    const npts = _.map(failedGroups, getNumPrincipaled);
    const meanNPT = mean(npts);
    const stdDevNPT = stdDev(npts, meanNPT);
    const onlyHigh = _.filter(npts, npt => npt > meanNPT + stdDevNPT);
    if (onlyHigh.length > 0) {
      return [sortByNPT, takeN(onlyHigh.length)];
    }

    const nptRatioEq = _.filter(
      failedGroups,
      group => group.length === getNumPrincipaled(group)
    );
    if (nptRatioEq.length > 0) {
      return [sortByNPTRatio, takeN(nptRatioEq.length)];
    }

    return [sortByDepth, takeAll];
  })();

  const sortedGroups = _.sortBy(_.values(failedGroups), [
    // Getting the right group is important but it's not precise.
    // We currently use the following metrics.
    //
    // 1. The number of obligations whose "principle types" are known (NPT).
    //    What this entails is that in an obligation such as `TYPE: TRAIT`,
    //    neither TYPE nor TRAIT is an unresolved type variable `_`.
    //
    //    We use this number, NPT, to find groups whose NPT is more than
    //    one standard deviation from the mean. This is useful when trait impls are
    //    macro generated for varying arities, larger arities have a high number of
    //    unresolved type variables.
    //
    //    The above is useful, until it isn't. Then, we can use tha ratio
    //    of group size vs the NPT. This favors smaller groups with more
    //    concrete types.
    //
    // 2. Depth of the group. Generally, deep obligations are "more interesting."
    sortStrategy,
  ]);

  const [importantGroups, rest] = firstFilter(sortedGroups);

  // The "Argus recommended" errors are shown expanded, and the
  // "others" are collapsed. Argus recommended errors are the ones
  // that failed or are ambiguous with a concrete type on the LHS.
  const [argusRecommendedLeaves, others] = _.partition(
    _.flatten(importantGroups),
    leaf => {
      const node = tree.node(leaf);
      if ("Goal" in node) {
        const goal = tree.goal(node.Goal);
        const result = tree.result(goal.result);
        return (
          !goal.isMainTv && (result === "no" || result === "maybe-overflow")
        );
      } else {
        // Leaves should only be goals...
        throw new Error(`Leaves should only be goals ${node}`);
      }
    }
  );

  const hiddenLeaves = _.concat(_.flatten(rest), others);
  const argusViews = invertViewWithRoots(argusRecommendedLeaves, tree);
  const otherViews = invertViewWithRoots(hiddenLeaves, tree);

  const LeafElement = ({ leaf }: { leaf: TreeViewWithRoot }) => (
    <DirRecursive level={[leaf.root]} getNext={mkGetChildren(leaf)} />
  );

  const recommendedSortedViews = tree.sortByRecommendedOrder(
    _.flatten(argusViews),
    v => v.root
  );
  const recommended = _.map(recommendedSortedViews, (leaf, i) => (
    <LeafElement key={i} leaf={leaf} />
  ));

  const fallbacks =
    others.length === 0 ? null : (
      <CollapsibleElement
        info={<span>Other failures ...</span>}
        Children={() =>
          _.map(otherViews, (leaf, i) => <LeafElement key={i} leaf={leaf} />)
        }
      />
    );

  const renderParams: TreeRenderParams = {
    Wrapper: ({
      n: _n,
      Child,
    }: {
      n: ProofNodeIdx;
      Child: React.ReactElement;
    }) => Child,
    styleEdges: false,
  };

  return (
    <TreeAppContext.TreeRenderContext.Provider value={renderParams}>
      {recommended}
      {fallbacks}
    </TreeAppContext.TreeRenderContext.Provider>
  );
};

export default BottomUp;
