import type { ProofNode } from "@argus/common/bindings";
import type { TreeRenderParams } from "@argus/common/communication";
import { TreeAppContext } from "@argus/common/context";
import {} from "@floating-ui/react";
import _ from "lodash";
import React, { useContext } from "react";

import { unpackProofNode } from "@argus/common/TreeInfo";
import { DirRecursive } from "./Directory";
import { WrapImplCandidates } from "./Wrappers";

const TopDown = ({ start }: { start?: ProofNode }) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const getGoalChildren = (kids: ProofNode[]) =>
    _.sortBy(kids, [k => tree.minInertiaOnPath(k)]);

  const getCandidateChildren = (kids: ProofNode[]) =>
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

  const getChildren = (idx: ProofNode) => {
    const node = unpackProofNode(idx);
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
          const startOpenP = (idx: ProofNode) =>
            pathToRootFromStart !== undefined &&
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
    Wrappers: [WrapImplCandidates],
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
