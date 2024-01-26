import { SerializedTree } from "@argus/common/bindings";
import classNames from "classnames";
import _ from "lodash";
import React, { PropsWithChildren, useContext, useState } from "react";

import {
  IcoChevronDown,
  IcoChevronRight,
  IcoDot,
  IcoTriangleDown,
  IcoTriangleRight,
} from "../utilities/icons";
import { TreeContext } from "./Context";
import "./Directory.css";
import { Node } from "./Node";

export type ElementPair = [React.ReactElement, React.ReactElement];

const defaultCollapseArrows: ElementPair = [
  <IcoChevronDown />,
  <IcoChevronRight />,
];

export const CollapsibleElement = ({
  info,
  icos = defaultCollapseArrows,
  indentChildren = false,
  children,
}: PropsWithChildren<{
  info: React.ReactElement;
  icos?: ElementPair;
  indentChildren?: boolean;
}>) => {
  const [isOpen, setIsOpen] = useState(false);
  const [openIco, closedIco] = icos;

  const toggleCollapse = (e: React.MouseEvent<HTMLElement>) => {
    e.preventDefault();
    setIsOpen(!isOpen);
  };

  const collapseCN = classNames({
    indent: indentChildren,
    collapsed: !isOpen,
  });

  return (
    <>
      <div className="DirNode" onClick={toggleCollapse}>
        {isOpen ? openIco : closedIco}
        <span className="information">{info}</span>
      </div>
      <div id="Collapsible" className={collapseCN}>
        {children}
      </div>
    </>
  );
};

export const DirNode = ({
  idx,
  styleEdge,
  children,
}: PropsWithChildren<{ idx: number; styleEdge: boolean }>) => {
  const tree = useContext(TreeContext)!;
  const node = tree.nodes[idx];

  const arrows: ElementPair = [<IcoTriangleDown />, <IcoTriangleRight />];
  const dots: ElementPair = [<IcoDot />, <IcoDot />];
  const icos = node.type === "result" ? dots : arrows;
  const info = <Node node={node} />;

  return (
    <CollapsibleElement info={info} icos={icos} indentChildren={true}>
      {children}
    </CollapsibleElement>
  );
};

export const DirRecursive = ({
  level,
  getNext,
  styleEdges,
}: {
  level: number[];
  getNext: (tree: SerializedTree, idx: number) => number[];
  styleEdges: boolean;
}) => {
  const tree = useContext(TreeContext)!;
  const node = tree.nodes[level[0]];
  const className = classNames({
    DirRecursive: true,
    "is-candidate": styleEdges && node?.type === "candidate",
    "is-subgoal": styleEdges && node?.type === "goal",
  });

  return (
    <div className={className}>
      {_.map(level, (current, i) => {
        const next = getNext(tree, current);
        return (
          <DirNode key={i} idx={current} styleEdge={styleEdges}>
            <DirRecursive
              level={next}
              getNext={getNext}
              styleEdges={styleEdges}
            />
          </DirNode>
        );
      })}
    </div>
  );
};
