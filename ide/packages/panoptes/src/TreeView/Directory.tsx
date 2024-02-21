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
  const [isOpen, setIsOpen] = useState(startOpen);
  const [openIco, closedIco] = icons;

  const toggleCollapse = (e: React.MouseEvent<HTMLElement>) => {
    e.preventDefault();
    setIsOpen(!isOpen);
  };

  const collapseCN = classNames("Collapsible", {
    indent: indentChildren,
    collapsed: !isOpen,
  });

  useEffect(() => {
    setIsOpen(startOpen || isOpen);
  }, [startOpen, isOpen]);

  return (
    <>
      <div className="DirNode" onClick={toggleCollapse}>
        <div className="toggle">
          {Children !== null ? (isOpen ? openIco : closedIco) : null}
        </div>
        <span className="information">{info}</span>
      </div>
      <div className={collapseCN}>
        {isOpen && Children !== null ? <Children /> : null}
      </div>
    </>
  );
};

type InfoWrapper = React.FC<{ n: ProofNodeIdx; Child: React.FC }>;

export const DirNode = ({
  idx,
  Children,
  Wrapper = ({ n: _, Child }) => <Child />,
}: {
  idx: number;
  Children: React.FC | null;
  Wrapper: InfoWrapper;
}) => {
  const tree = useContext(TreeContext)!;
  const node = tree.node(idx);

  const arrows: ElementPair = [<IcoTriangleDown />, <IcoTriangleRight />];
  const dots: ElementPair = [<IcoDot />, <IcoDot />];
  const icons = "Result" in node ? dots : arrows;
  const info = <Wrapper n={idx} Child={() => <Node node={node} />} />;

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
  styleEdges,
  Wrapper = ({ n: _, Child }) => <Child />,
}: {
  level: ProofNodeIdx[];
  getNext: (idx: ProofNodeIdx) => ProofNodeIdx[];
  styleEdges: boolean;
  Wrapper?: InfoWrapper;
}) => {
  const tree = useContext(TreeContext)!;
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
        return (
          <DirNode
            key={i}
            idx={current}
            Wrapper={Wrapper}
            Children={
              next.length > 0
                ? () => (
                    <DirRecursive
                      level={next}
                      getNext={getNext}
                      styleEdges={styleEdges}
                      Wrapper={Wrapper}
                    />
                  )
                : null
            }
          />
        );
      })}
    </div>
  );
};
