import { ProofNodeIdx } from "@argus/common/bindings";
import {
  FloatingPortal,
  useClick,
  useFloating,
  useInteractions,
} from "@floating-ui/react";
import classNames from "classnames";
import _ from "lodash";
import React, { useContext, useState } from "react";

import { IcoTreeDown } from "../Icons";
import { TreeContext } from "./Context";
import { DirRecursive } from "./Directory";
import Graph from "./Graph";
import "./TopDown.css";

export const WrapTreeIco = ({
  n,
  Child,
}: {
  n: ProofNodeIdx;
  Child: React.FC;
}) => {
  const [isHovered, setIsHovered] = useState(false);
  const [isOpen, setIsOpen] = useState(false);
  const { refs, floatingStyles, context } = useFloating({
    open: isOpen,
    onOpenChange: setIsOpen,
  });

  const click = useClick(context);
  const { getReferenceProps, getFloatingProps } = useInteractions([click]);

  return (
    <span
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <Child />
      <span
        className="tree-toggle"
        ref={refs.setReference}
        {...getReferenceProps()}
      >
        {(isHovered || isOpen) && <IcoTreeDown />}
        {isOpen && (
          <FloatingPortal>
            <div
              className={classNames("floating", "floating-graph")}
              ref={refs.setFloating}
              style={floatingStyles}
              {...getFloatingProps()}
            >
              <Graph root={n} />
            </div>
          </FloatingPortal>
        )}
      </span>
    </span>
  );
};

const TopDown = () => {
  const tree = useContext(TreeContext)!;

  const getGoalChildren = (kids: ProofNodeIdx[]) => {
    // Sort the candidates by the #infer vars / height of the tree
    return _.sortBy(kids, k => {
      const inferVars = tree.inferVars(k);
      const height = tree.maxHeigh(k);
      return inferVars / height;
    });
  };

  const getCandidateChildren = (kids: ProofNodeIdx[]) => {
    return _.sortBy(
      kids,
      k => {
        switch (tree.result(k) ?? "yes") {
          case "no":
            return 0;
          case "maybe-overflow":
            return 1;
          case "maybe-ambiguity":
            return 2;
          case "yes":
            return 3;
        }
      },
      k => {
        const node = tree.node(k);
        "Goal" in node && node.Goal.data.isLhsTyVar ? 1 : 0;
      }
    );
  };

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
  return (
    <DirRecursive
      level={[tree.root]}
      getNext={getChildren}
      styleEdges={true}
      Wrapper={WrapTreeIco}
    />
  );
};

export default TopDown;
