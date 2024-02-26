import { ProofNodeIdx, TreeTopology } from "@argus/common/bindings";
import _ from "lodash";
import React, { useContext } from "react";

import { TreeContext } from "./Context";
import { CollapsibleElement, DirRecursive } from "./Directory";
import { TreeInfo, TreeView } from "./TreeInfo";

class TopologyBuilder {
  private topo: TreeTopology;
  constructor() {
    this.topo = { children: {}, parent: {} };
  }

  get topology() {
    return this.topo;
  }

  public add(parent: ProofNodeIdx, child: ProofNodeIdx) {
    if (!this.topo.children[parent]) {
      this.topo.children[parent] = [];
    }
    this.topo.children[parent]!.push(child);
    this.topo.parent[child] = parent;
  }
}

/**
 *
 * @param inputArray array to generate windows across
 * @param size the size of window to generate
 * @returns an array of windows of size `size` across `inputArray`. Final window may be smaller than `size`.
 */
function toWindows<T>(arr: T[], size: number): T[][] {
  let windowed = [];
  for (let idx = 0; idx < arr.length; idx += size) {
    windowed.push(_.slice(arr, idx, idx + size));
  }
  return windowed;
}

function toPairs<T>(inputArray: T[]): [T, T][] {
  return toWindows(inputArray, 2) as any;
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
): Array<TreeView & { root: ProofNodeIdx }> {
  const groupedLeaves = _.groupBy(leaves, leaf => {
    const node = tree.node(leaf);
    if ("Goal" in node) {
      console.debug("Grouping by goalidx", node.Goal[0]);
      return node.Goal[0];
    } else {
      throw new Error("Leaves should only be goals");
    }
  });

  console.debug("groupedLeaves", groupedLeaves);

  const groups: ProofNodeIdx[][] = _.values(groupedLeaves);

  return _.map(groups, group => {
    const builder = new TopologyBuilder();
    const root = group[0];
    const parents = _.map(leaves, leaf => tree.parent(leaf));
    const pathsToRoot = _.map(
      _.compact(parents),
      parent => tree.pathToRoot(parent).path
    );
    _.forEach(pathsToRoot, path => {
      builder.add(root, path[0]);
      _.forEach(toPairs(path), ([a, b]) => {
        builder.add(a, b);
      });
    });

    return { topology: builder.topology, root };
  });
}

const BottomUp = () => {
  const tree = useContext(TreeContext)!;
  const mkGetChildren = (view: TreeView) => (idx: ProofNodeIdx) =>
    view.topology.children[idx] ?? [];

  const leaves = _.map(tree.errorNodesRecommendedOrder(), leaf => {
    let curr: ProofNodeIdx | undefined = leaf;
    while (curr !== undefined && !("Goal" in tree.node(curr))) {
      curr = tree.parent(curr);
    }
    return curr;
  });

  const invertedViews = invertViewWithRoots(_.compact(leaves), tree);

  // The "Argus recommended" errors are shown expanded, and the
  // "others" are collapsed. Argus recommended errors are the ones
  // that failed or are ambiguous with a concrete type on the LHS.
  const [argusRecommended, others] = _.partition(invertedViews, view => {
    const node = tree.node(view.root);
    if ("Goal" in node) {
      const goal = tree.goal(node.Goal[0]);
      const result = tree.result(node.Goal[1]);
      return result === "no" || result === "maybe-overflow" || !goal.isLhsTyVar;
    } else {
      // Leaves should only be goals...
      return false;
    }
  });

  const LeafElement = ({
    leaf,
  }: {
    leaf: TreeView & { root: ProofNodeIdx };
  }) => (
    <DirRecursive
      level={[leaf.root]}
      getNext={mkGetChildren(leaf)}
      styleEdges={false}
    />
  );

  const recommended = _.map(argusRecommended, (leaf, i) => (
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
