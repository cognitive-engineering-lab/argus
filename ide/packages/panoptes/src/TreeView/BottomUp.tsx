import { TreeInfo, TreeView } from "@argus/common/TreeInfo";
import { ProofNodeIdx, TreeTopology } from "@argus/common/bindings";
import { TreeRenderParams } from "@argus/common/communication";
import { TreeAppContext } from "@argus/common/context";
import { EvaluationMode } from "@argus/common/lib";
import { PrintGoal } from "@argus/print/lib";
import _ from "lodash";
import React, { useContext } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";

import "./BottomUp.css";
import { CollapsibleElement, DirRecursive } from "./Directory";

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
export function invertViewWithRoots(
  leaves: ProofNodeIdx[],
  tree: TreeInfo
): TreeViewWithRoot[] {
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

const RenderEvaluationViews = ({
  recommended,
  others,
  mode,
}: {
  recommended: TreeViewWithRoot[];
  others: TreeViewWithRoot[];
  mode: "rank" | "random";
}) => {
  const nodeToString = (node: React.ReactNode) => {
    const div = document.createElement("div");
    const root = createRoot(div);
    flushSync(() => root.render(node));
    return div.innerText;
  };

  const tree = useContext(TreeAppContext.TreeContext)!;
  let together = _.concat(recommended, others);

  if (mode === "random") {
    together = _.shuffle(together);
  }

  const [goals, setGoals] = React.useState<string[]>([]);
  const nodeList: React.ReactNode[] = _.compact(
    _.map(together, (leaf, i) => {
      const node = tree.node(leaf.root);
      return "Goal" in node ? (
        <PrintGoal key={i} o={tree.goal(node.Goal)} />
      ) : null;
    })
  );

  React.useEffect(() => {
    // run outside of react lifecycle
    window.setTimeout(() => setGoals(_.map(nodeList, nodeToString)));
  }, []);

  return (
    <div className="BottomUpArea">
      {_.map(goals, (s, i) => (
        <div key={i} className="EvalGoal" data-rank={i} data-goal={s}>
          {s}
        </div>
      ))}
    </div>
  );
};

/**
 * The actual entry point for rendering the bottom up view. All others are used in testing or evaluation.
 */
export const RenderBottomUpViews = ({
  recommended,
  others,
}: {
  recommended: TreeViewWithRoot[];
  others: TreeViewWithRoot[];
}) => {
  const mkGetChildren = (view: TreeView) => (idx: ProofNodeIdx) =>
    view.topology.children[idx] ?? [];

  const mkTopLevel = (views: TreeViewWithRoot[]) =>
    _.map(views, (leaf, i) => (
      <DirRecursive key={i} level={[leaf.root]} getNext={mkGetChildren(leaf)} />
    ));

  const argusViews = mkTopLevel(recommended);
  const fallbacks =
    others.length === 0 ? null : (
      <CollapsibleElement
        info={<span id="hidden-failure-list">Other failures ...</span>}
        Children={() => mkTopLevel(others)}
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
      <div id="recommended-failure-list">{argusViews}</div>
      {fallbacks}
    </TreeAppContext.TreeRenderContext.Provider>
  );
};

export function liftTo(
  tree: TreeInfo,
  idx: ProofNodeIdx,
  target: "Goal" | "Candidate"
) {
  let curr: ProofNodeIdx | undefined = idx;
  while (curr !== undefined && !(target in tree.node(curr))) {
    curr = tree.parent(curr);
  }
  return curr;
}

// A bit of a hack to allow the evaluation script to render the bottom up view differently.
export const BottomUpImpersonator = ({
  recommended,
  others,
  mode,
}: {
  recommended: TreeViewWithRoot[];
  others: TreeViewWithRoot[];
  mode: EvaluationMode;
}) => {
  return mode === "release" ? (
    <RenderBottomUpViews recommended={recommended} others={others} />
  ) : (
    <RenderEvaluationViews
      recommended={recommended}
      others={others}
      mode={mode}
    />
  );
};
