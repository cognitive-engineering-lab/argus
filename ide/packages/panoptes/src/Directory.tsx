import { SerializedTree } from "@argus/common/types";
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

export const DirNode = ({
  idx,
  children,
}: PropsWithChildren<{ idx: number }>) => {
  const [isOpen, setIsOpen] = useState(false);

  const tree = useContext(TreeContext)!;
  const node = tree.nodes[idx];

  const [openIco, closedIco] = ["▼", "▶"];
  const toggleCollapse = (e: React.MouseEvent<HTMLElement>) => {
    e.preventDefault();
    setIsOpen(!isOpen);
  };

  return (
    <>
      <div className="DirNode" onClick={toggleCollapse}>
        <span>{isOpen ? openIco : closedIco}</span>
        <span className="information">{node}</span>
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
}: {
  level: number[];
  getNext: (tree: SerializedTree, idx: number) => number[];
}) => {
  const tree = useContext(TreeContext)!;
  const topo = tree.topology;
  return (
    <div className="DirRecursive">
      {_.map(level, (current, i) => {
        const next = getNext(tree, current);

        return (
          <DirNode key={i} idx={current}>
            <DirRecursive level={next} getNext={getNext} />
          </DirNode>
        );
      })}
    </div>
  );
};
