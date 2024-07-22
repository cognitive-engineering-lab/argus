import type { ProofNodeIdx, SetHeuristic } from "@argus/common/bindings";
import { AppContext, TreeAppContext } from "@argus/common/context";
import classNames from "classnames";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { useContext, useState } from "react";
import { MiniBufferDataStore } from "../signals";

import type { TreeView } from "@argus/common/TreeInfo";
import {
  BottomUpImpersonator,
  BottomUpRenderParams,
  type TreeViewWithRoot,
  invertViewWithRoots
} from "./BottomUp";
import { CollapsibleElement, DirRecursive } from "./Directory";
import "./Subsets.css";
import { IcoComment } from "@argus/print/Icons";

/**
 * Define the heuristic used for inertia in the system. Previously we were
 * using `momentum / velocity` but this proved too sporatic. Some proof trees
 * were deep, needlessely, and this threw a wrench in the order.
 */
const setInertia = (set: SetHeuristic) => set.momentum;
export const sortedSubsets = (sets: SetHeuristic[]) =>
  _.sortBy(sets, setInertia);

export const RenderBottomUpSets = ({
  views
}: {
  views: {
    tree: TreeViewWithRoot[];
    inertia: number;
    velocity: number;
    momentum: number;
  }[];
}) => {
  const mkGetChildren = (view: TreeView) => (idx: ProofNodeIdx) =>
    view.topology.children[idx] ?? [];

  const MkLevel = observer(
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

  const argusRecommends = <MkLevel {..._.head(views)!} />;
  const tail = _.tail(views);

  const otherLabel = "Other failures";
  const fallbacks =
    tail.length === 0 ? null : (
      <CollapsibleElement
        info={<span id="hidden-failure-list">{otherLabel} ...</span>}
        Children={() => _.map(tail, (v, i) => <MkLevel {...v} key={i} />)}
      />
    );

  return (
    <TreeAppContext.TreeRenderContext.Provider value={BottomUpRenderParams}>
      <p>
        Argus recommends investigating these failed oblgiations. Click on ’
        {otherLabel}‘ below to see other failed obligations.
      </p>
      <div id="recommended-failure-list">{argusRecommends}</div>
      {fallbacks}
    </TreeAppContext.TreeRenderContext.Provider>
  );
};

const FailedSubsets = () => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const evaluationMode =
    useContext(AppContext.ConfigurationContext)?.evalMode ?? "release";
  const sets = sortedSubsets(tree.failedSets);

  const makeSets = (sets: SetHeuristic[]) =>
    _.map(sets, h => {
      return {
        tree: invertViewWithRoots(
          _.map(h.goals, g => g.idx),
          tree
        ),
        inertia: setInertia(h),
        velocity: h.velocity,
        momentum: h.momentum
      };
    });

  if (evaluationMode === "release") {
    return <RenderBottomUpSets views={makeSets(sets)} />;
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

export default FailedSubsets;
