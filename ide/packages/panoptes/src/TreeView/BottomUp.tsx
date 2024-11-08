import type { EvaluationMode } from "@argus/common/lib";
import { PrintGoal } from "@argus/print/lib";
import React, { useContext, useState } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";

import type { ProofNodeIdx, SetHeuristic } from "@argus/common/bindings";
import type { TreeRenderParams } from "@argus/common/communication";
import { AppContext, TreeAppContext } from "@argus/common/context";
import classNames from "classnames";
import _ from "lodash";
import { observer } from "mobx-react";
import { MiniBufferDataStore } from "../signals";

import {
  TreeInfo,
  type TreeView,
  type TreeViewWithRoot,
  invertViewWithRoots
} from "@argus/common/TreeInfo";
import { IcoComment } from "@argus/print/Icons";
import { WrapImplCandidates, mkJumpToTopDownWrapper } from "./Wrappers";

import { CollapsibleElement, DirRecursive } from "./Directory";
import "./BottomUp.css";
import { TyCtxt } from "@argus/print/context";

const RenderEvaluationViews = ({
  recommended,
  others,
  mode
}: {
  recommended: TreeViewWithRoot[];
  others: TreeViewWithRoot[];
  mode: "rank" | "random";
}) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const tyCtxt = useContext(TyCtxt)!;

  const nodeToString = (node: React.ReactNode) => {
    const div = document.createElement("div");
    const root = createRoot(div);
    flushSync(() => root.render(node));
    return div.innerText;
  };

  let together = _.concat(recommended, others);

  if (mode === "random") {
    together = _.shuffle(together);
  }

  const [goals, setGoals] = React.useState<string[]>([]);
  const nodeList: React.ReactNode[] = _.compact(
    _.map(together, (leaf, i) => {
      const node = tree.node(leaf.root);
      return "Goal" in node ? (
        <TyCtxt.Provider value={tyCtxt} key={i}>
          <PrintGoal o={tree.goal(node.Goal)} />
        </TyCtxt.Provider>
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
  others
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

  return (
    <>
      <div id="recommended-failure-list">{argusViews}</div>
      {fallbacks}
    </>
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
  mode
}: {
  recommended: TreeViewWithRoot[];
  others: TreeViewWithRoot[];
  mode: EvaluationMode;
}) =>
  mode === "release" ? (
    <RenderBottomUpViews recommended={recommended} others={others} />
  ) : (
    <RenderEvaluationViews
      recommended={recommended}
      others={others}
      mode={mode}
    />
  );

export const sortedSubsets = (sets: SetHeuristic[]) =>
  _.sortBy(sets, TreeInfo.setInertia);

const mkGetChildren = (view: TreeView) => (idx: ProofNodeIdx) =>
  view.topology.children[idx] ?? [];

const GroupedFailures = observer(
  (views: {
    tree: TreeViewWithRoot[];
    inertia: number;
    momentum: number;
    velocity: number;
  }) => {
    if (views.tree.length === 0) {
      return null;
    }

    const [hovered, setHovered] = useState(false);
    const cn = classNames("FailingSet", { "is-hovered": hovered });

    // If there is only a single predicate, no need to provide all the
    // extra information around "grouped predicate sets".
    if (views.tree.length === 1) {
      return (
        <div className={cn}>
          <DirRecursive
            level={[views.tree[0].root]}
            getNext={mkGetChildren(views.tree[0])}
          />
        </div>
      );
    }

    const onHover = () => {
      MiniBufferDataStore.set({
        kind: "argus-note",
        data: (
          <p>
            The outlined obligations must be resolved <b>together</b>
          </p>
        )
      });
      setHovered(true);
    };

    const onNoHover = () => {
      MiniBufferDataStore.reset();
      setHovered(false);
    };

    return (
      <div className={cn}>
        <IcoComment onMouseEnter={onHover} onMouseLeave={onNoHover} />
        {_.map(views.tree, (leaf, i) => (
          <DirRecursive
            key={i}
            level={[leaf.root]}
            getNext={mkGetChildren(leaf)}
          />
        ))}
      </div>
    );
  }
);

export const RenderBottomUpSets = ({
  views,
  jumpTo
}: {
  jumpTo: (n: ProofNodeIdx) => void;
  views: {
    tree: TreeViewWithRoot[];
    inertia: number;
    velocity: number;
    momentum: number;
  }[];
}) => {
  const argusRecommends = <GroupedFailures {..._.head(views)!} />;
  const tail = _.tail(views);

  const otherLabel = "Other failures";
  const fallbacks =
    tail.length === 0 ? null : (
      <CollapsibleElement
        info={<span id="hidden-failure-list">{otherLabel} ...</span>}
        Children={() =>
          _.map(tail, (v, i) => <GroupedFailures {...v} key={i} />)
        }
      />
    );

  const SubsetRenderParams: TreeRenderParams = {
    Wrappers: [WrapImplCandidates, mkJumpToTopDownWrapper(jumpTo)],
    styleEdges: false
  };

  return (
    <TreeAppContext.TreeRenderContext.Provider value={SubsetRenderParams}>
      <p>
        Argus recommends investigating these failed obligations. Click on ’
        {otherLabel}‘ below to see other failed obligations.
      </p>
      <div id="recommended-failure-list">{argusRecommends}</div>
      {fallbacks}
    </TreeAppContext.TreeRenderContext.Provider>
  );
};

const BottomUp = ({
  jumpToTopDown
}: { jumpToTopDown: (n: ProofNodeIdx) => void }) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const evaluationMode =
    useContext(AppContext.ConfigurationContext)?.evalMode ?? "release";
  const sets = sortedSubsets(tree.failedSets());

  const makeSets = (sets: SetHeuristic[]) =>
    _.map(sets, h => {
      return {
        tree: invertViewWithRoots(
          _.map(h.goals, g => g.idx),
          tree
        ),
        inertia: TreeInfo.setInertia(h),
        velocity: h.velocity,
        momentum: h.momentum
      };
    });

  if (evaluationMode === "release") {
    return <RenderBottomUpSets jumpTo={jumpToTopDown} views={makeSets(sets)} />;
  }

  const flattenSets = (sets: SetHeuristic[]) =>
    _.flatMap(sets, h =>
      invertViewWithRoots(
        _.map(h.goals, g => g.idx),
        tree
      )
    );

  // Flatten all the sets and return them as a list.
  const suggestedPredicates = flattenSets(_.slice(sets, 0, 3));
  const others = flattenSets(_.slice(sets, 3));
  return (
    <BottomUpImpersonator
      recommended={suggestedPredicates}
      others={others}
      mode={evaluationMode}
    />
  );
};

export default BottomUp;
