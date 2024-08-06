import type { ProofNodeIdx } from "@argus/common/bindings";
import type { TreeRenderParams } from "@argus/common/communication";
import { TreeAppContext } from "@argus/common/context";
import {} from "@floating-ui/react";
import _ from "lodash";
import React, { useContext } from "react";

import { DirRecursive } from "./Directory";

const TopDown = ({ start }: { start?: ProofNodeIdx }) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const getGoalChildren = (kids: ProofNodeIdx[]) =>
    _.sortBy(kids, [k => tree.minInertiaOnPath(k)]);

  const getCandidateChildren = (kids: ProofNodeIdx[]) =>
    _.sortBy(_.uniq(kids), [
      k => {
        switch (tree.nodeResult(k)) {
          case "no":
            return tree.minInertiaOnPath(k);
          case "maybe-overflow":
            return tree.minInertiaOnPath(k) + 10_000;
          case "maybe-ambiguity":
            return tree.minInertiaOnPath(k) + 100_000;
          default:
            return 1_000_000;
        }
      }
    ]);

  const getChildren = (idx: ProofNodeIdx) => {
    const node = tree.node(idx);
    const kids = tree.children(idx);
    if ("Goal" in node) {
      return getGoalChildren(kids);
    } else if ("Candidate" in node) {
      return getCandidateChildren(kids);
    } else {
      return [];
    }
  };

  const ops =
    start === undefined
      ? undefined
      : (() => {
          const pathToRootFromStart = tree.pathToRoot(start);
          const startOpenP = (idx: ProofNodeIdx) =>
            _.includes(pathToRootFromStart.pathInclusive, idx);
          const onMount = () => {
            const element = document.querySelector<HTMLSpanElement>(
              `.proof-node-${start}`
            );
            element?.scrollIntoView({
              block: "start",
              inline: "nearest",
              behavior: "smooth"
            });
          };
          return {
            startOpenP,
            onMount
          };
        })();

  const renderParams: TreeRenderParams = {
    styleEdges: true,
    ...ops
  };

  return (
    <TreeAppContext.TreeRenderContext.Provider value={renderParams}>
      <DirRecursive level={[tree.root]} getNext={getChildren} />
    </TreeAppContext.TreeRenderContext.Provider>
  );
};

export default TopDown;
