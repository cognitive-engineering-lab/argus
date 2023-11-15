import { SerializedTree } from "@argus/common/types";
import { TreeDescription, TreeTopology } from "@argus/common/types";
import _ from "lodash";
import { observer } from "mobx-react";
import React, {
  MouseEvent,
  MouseEventHandler,
  useCallback,
  useContext,
  useState,
} from "react";
// @ts-ignore
import Tree, { TreeNodeDatum } from "react-d3-tree";
import ReactDOM from "react-dom";

import { ActiveContext } from "./Context";
import "./TreeArea.css";

const useCenteredTree = (
  defaultTranslate = { x: 0, y: 0 }
): [{ x: number; y: number }, (elem: HTMLInputElement) => void] => {
  const [translate, setTranslate] = useState(defaultTranslate);

  const containerRef = useCallback((containerElem: HTMLInputElement) => {
    if (containerElem !== null) {
      const { width, height } = containerElem.getBoundingClientRect();
      setTranslate({ x: width / 2, y: height / 5 });
    }
  }, []);

  return [translate, containerRef];
};

// --------------------
// Components

const TreeNode = observer(
  ({
    nodeDatum,
    toggleNode,
    hoverNode,
  }: {
    nodeDatum: TreeNodeDatum;
    toggleNode: MouseEventHandler<SVGCircleElement>;
    hoverNode: MouseEventHandler<SVGCircleElement>;
  }) => {
    return (
      <g>
        <circle
          data-sidx={nodeDatum.name}
          r="10"
          onClick={toggleNode}
          onMouseEnter={hoverNode}
        />
        <text fill="black" strokeWidth="1" x="20">
          {nodeDatum.name}
        </text>
      </g>
    );
  }
);

const topologyToTreeData = (
  topology: TreeTopology<number>,
  idx: number
): TreeNodeDatum => {
  let kids = topology.children[idx];
  let obj: TreeNodeDatum = {
    name: idx,
  };

  if (kids) {
    let kobjs = _.map(kids, k => topologyToTreeData(topology, k));
    obj = {
      ...obj,
      children: kobjs,
    };
  }

  return obj;
};

const TreeArea = observer(({ tree }: { tree: SerializedTree }) => {
  const activeContext = useContext(ActiveContext)!;
  const [translate, containerRef] = useCenteredTree();

  const descr = tree.descr;
  const topology = tree.topology;
  const data = topologyToTreeData(topology, descr.root);

  const handleNodeHover = (evnt: MouseEvent<SVGCircleElement>) => {
    let sid = evnt.currentTarget.dataset.sidx;
    if (sid !== undefined) {
      let idx = +sid as number;
      activeContext.setActiveNode(idx);
      console.log("Hovered node name", idx);
    } else {
      console.warn("Hovered node name is undefined", evnt.currentTarget);
    }
  };

  const customRender = (rd3tProps: any) => {
    return <TreeNode hoverNode={handleNodeHover} {...rd3tProps} />;
  };

  return (
    <div className="TreeArea" ref={containerRef}>
      <Tree
        data={data}
        renderCustomNodeElement={customRender}
        orientation="vertical"
        translate={translate}
      />
    </div>
  );
});

export default TreeArea;
