import { SerializedTree } from "@argus/common/types";
import classNames from "classnames";
import _ from "lodash";
import { observer } from "mobx-react";
import React, {
  PropsWithChildren,
  createContext,
  useContext,
  useState,
} from "react";

import { TreeContext } from "./Context";
import "./Directory.css";
import { nodeContent } from "./utilities";

export const DirNode = ({
  idx,
  styleEdge,
  children,
}: PropsWithChildren<{ idx: number, styleEdge: boolean }>) => {
  const [isOpen, setIsOpen] = useState(false);
  const tree = useContext(TreeContext)!;
  const node = tree.nodes[idx];

  const arrows = ["▼", "▶"];
  const dots = ["•", "•"];
  const [openIco, closedIco] = node.type === 'result' ? dots : arrows;;

  const toggleCollapse = (e: React.MouseEvent<HTMLElement>) => {
    e.preventDefault();
    setIsOpen(!isOpen);
  };

  const content = nodeContent(node);

  return (
    <>
      <div className="DirNode" onClick={toggleCollapse}>
        <span>{isOpen ? openIco : closedIco}</span>
        <span className="information">{content}</span>
      </div>
      <div id="Collapsible" className={isOpen ? "" : "collapsed"}>
        {children}
      </div>
    </>
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
