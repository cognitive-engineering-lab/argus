import type { ProofNodeIdx } from "@argus/common/bindings";
import { AppContext, TreeAppContext } from "@argus/common/context";
import {
  IcoChevronDown,
  IcoChevronRight,
  IcoDot,
  IcoTriangleDown,
  IcoTriangleRight
} from "@argus/print/Icons";
import classNames from "classnames";
import _ from "lodash";
import React, { useContext, useEffect, useState } from "react";

import "./Directory.css";
import Attention from "@argus/print/Attention";
import { Node } from "./Node";
import { WrapNode } from "./Wrappers";

export type ElementPair = [React.ReactElement, React.ReactElement];

const defaultCollapseArrows: ElementPair = [
  <IcoChevronDown />,
  <IcoChevronRight />
];

export const CollapsibleElement = ({
  info,
  icons = defaultCollapseArrows,
  indentChildren = false,
  startOpen = false,
  Children
}: {
  info: React.ReactElement;
  icons?: ElementPair;
  indentChildren?: boolean;
  startOpen?: boolean;
  Children: React.FC | null;
}) => {
  const config = useContext(AppContext.ConfigurationContext)!;
  const openByDefault = startOpen || config.evalMode !== "release";

  const [openIco, closedIco] = icons;
  const [isOpen, setIsOpen] = useState(openByDefault);
  const [children, setChildren] = useState<React.ReactElement | undefined>(
    undefined
  );

  useEffect(() => {
    if (children === undefined && Children !== null && isOpen) {
      setChildren(<Children />);
    }
  }, [isOpen]);

  const toggleCollapse = (e: React.MouseEvent<HTMLElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsOpen(!isOpen);
  };

  const collapseCN = classNames("DirNodeChildren", {
    indent: indentChildren,
    collapsed: !isOpen
  });

  const LabelWrapper = startOpen ? Attention : React.Fragment;

  return (
    <div className="DirNode">
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: TODO */}
      <div className="DirNodeLabel" onClick={toggleCollapse}>
        <div className="toggle">
          {Children !== null ? (isOpen ? openIco : closedIco) : null}
        </div>
        <div className="label">
          <LabelWrapper>{info}</LabelWrapper>
        </div>
      </div>
      <div className={collapseCN}>{children}</div>
    </div>
  );
};

export const DirNode = ({
  idx,
  Children
}: {
  idx: number;
  Children: React.FC | null;
}) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const { Wrappers, startOpenP } = useContext(TreeAppContext.TreeRenderContext);
  const node = tree.node(idx);

  const arrows: ElementPair = [<IcoTriangleDown />, <IcoTriangleRight />];
  const dots: ElementPair = [<IcoDot />, <IcoDot />];
  const icons = "Result" in node ? dots : arrows;
  const infoChild = (
    <span className={`proof-node-${idx}`}>
      <Node node={node} />
    </span>
  );

  const info =
    Wrappers === undefined ? (
      infoChild
    ) : (
      <WrapNode wrappers={Wrappers} n={idx}>
        {infoChild}
      </WrapNode>
    );
  const startOpen = startOpenP ? startOpenP(idx) : false;

  if (idx === 0) {
    console.warn("StartOpen", startOpen);
  }

  return (
    <CollapsibleElement
      info={info}
      icons={icons}
      indentChildren={true}
      Children={Children}
      startOpen={startOpen}
    />
  );
};

export const DirRecursive = ({
  level,
  getNext
}: {
  level: ProofNodeIdx[];
  getNext: (idx: ProofNodeIdx) => ProofNodeIdx[];
}) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const { styleEdges, onMount } = useContext(TreeAppContext.TreeRenderContext);
  const node = tree.node(level[0]);
  const className = classNames("DirRecursive", {
    "is-candidate": styleEdges && "Candidate" in node,
    "is-subgoal": styleEdges && "Goal" in node,
    "generic-edge": !styleEdges
  });

  useEffect(() => {
    onMount ? onMount() : {};
  }, []);

  return (
    <div className={className}>
      {_.map(level, (current, i) => {
        const next = getNext(current);
        const Children =
          next.length > 0
            ? () => <DirRecursive level={next} getNext={getNext} />
            : null;
        return <DirNode key={i} idx={current} Children={Children} />;
      })}
    </div>
  );
};
