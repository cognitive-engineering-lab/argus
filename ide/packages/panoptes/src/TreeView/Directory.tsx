import { ProofNodeIdx } from "@argus/common/bindings";
import classNames from "classnames";
import _ from "lodash";
import React, { useContext, useEffect, useState } from "react";

import {
  IcoChevronDown,
  IcoChevronRight,
  IcoDot,
  IcoTriangleDown,
  IcoTriangleRight,
} from "../Icons";
import { AppContext, TreeAppContext } from "../utilities/context";
import "./Directory.css";
import { Node } from "./Node";

export type ElementPair = [React.ReactElement, React.ReactElement];

const defaultCollapseArrows: ElementPair = [
  <IcoChevronDown />,
  <IcoChevronRight />,
];

export const CollapsibleElement = ({
  info,
  icons = defaultCollapseArrows,
  indentChildren = false,
  startOpen = false,
  Children,
}: {
  info: React.ReactElement;
  icons?: ElementPair;
  indentChildren?: boolean;
  startOpen?: boolean;
  Children: React.FC | null;
}) => {
  const config = useContext(AppContext.ConfigurationContext)!;
  const openByDefault = startOpen || config.evalMode !== "release";

  const [isOpen, setIsOpen] = useState(openByDefault);
  const [openIco, closedIco] = icons;
  let [children, setChildren] = useState<React.ReactElement | undefined>(
    undefined
  );
  useEffect(() => {
    if (children === undefined && Children !== null && isOpen) {
      setChildren(<Children />);
    }
  }, [isOpen]);

  useEffect(() => {
    setIsOpen(startOpen || isOpen);
  }, [startOpen, isOpen]);

  const toggleCollapse = (e: React.MouseEvent<HTMLElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsOpen(!isOpen);
  };

  const collapseCN = classNames("DirNodeChildren", {
    indent: indentChildren,
    collapsed: !isOpen,
  });

  return (
    <div className="DirNode">
      <div className="DirNodeLabel" onClick={toggleCollapse}>
        <div className="toggle">
          {Children !== null ? (isOpen ? openIco : closedIco) : null}
        </div>
        <div className="label">{info}</div>
      </div>
      <div className={collapseCN}>{children}</div>
    </div>
  );
};

export type InfoWrapper = React.FC<{
  n: ProofNodeIdx;
  Child: React.ReactElement;
}>;
export interface TreeRenderParams {
  Wrapper?: InfoWrapper;
  styleEdges?: boolean;
}

export const DirNode = ({
  idx,
  Children,
}: {
  idx: number;
  Children: React.FC | null;
}) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const { Wrapper } = useContext(TreeAppContext.TreeRenderContext);
  const node = tree.node(idx);

  const arrows: ElementPair = [<IcoTriangleDown />, <IcoTriangleRight />];
  const dots: ElementPair = [<IcoDot />, <IcoDot />];
  const icons = "Result" in node ? dots : arrows;
  const infoChild = <Node node={node} />;
  const info = Wrapper ? <Wrapper n={idx} Child={infoChild} /> : infoChild;

  return (
    <CollapsibleElement
      info={info}
      icons={icons}
      indentChildren={true}
      Children={Children}
    />
  );
};

export const DirRecursive = ({
  level,
  getNext,
}: {
  level: ProofNodeIdx[];
  getNext: (idx: ProofNodeIdx) => ProofNodeIdx[];
}) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const { styleEdges } = useContext(TreeAppContext.TreeRenderContext);
  const node = tree.node(level[0]);
  const className = classNames("DirRecursive", {
    "is-candidate": styleEdges && "Candidate" in node,
    "is-subgoal": styleEdges && "Goal" in node,
    "generic-edge": !styleEdges,
  });

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
