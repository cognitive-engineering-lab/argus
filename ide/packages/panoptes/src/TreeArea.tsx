import { SerializedTree } from "@argus/common/types";
import { TreeTopology } from "@argus/common/types";
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

import { ActiveContext, TreeContext } from "./Context";
import { nodeContent } from "./utilities";
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

const calculateTextSize = (text: string): [number, number] => {
  // You can adjust the padding as needed
  const padding = 5;
  const textLength = text.length;
  const width = textLength * 8 + padding * 2;
  const height = 40; // Set the desired height

  return [width, height];
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
    toggleNode: MouseEventHandler<SVGRectElement>;
    hoverNode: MouseEventHandler<SVGRectElement>;
  }) => {
    const treeContext = useContext(TreeContext)!;
    const idx = nodeDatum.name as number;
    const label = nodeContent(treeContext.nodes[idx]);
    const [width, height] = calculateTextSize(label);

    // x={x + width / 2}
    // y={y + height / 2}
    return (
      <g>
        <rect
          data-sidx={idx}
          x={-width / 2} y={-height / 2}
          rx="5" ry="5"
          width={width} height={height}
          strokeWidth="0.5"
          fill="#f3f3f3"
          onClick={toggleNode}
          onMouseEnter={hoverNode}
        />
        <text
          dominant-baseline="middle"
          text-anchor="middle"
          fontFamily="monospace"
          fill="black"
          strokeWidth="0.25"
        >
          {label}
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

  const topology = tree.topology;
  const data = topologyToTreeData(topology, tree.root);

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

  const nodeSize = { x: 250, y: 100 };

  return (
    <div className="TreeArea" ref={containerRef}>
      <Tree
        data={data}
        renderCustomNodeElement={customRender}
        orientation="vertical"
        translate={translate}
        nodeSize={nodeSize}
      />
    </div>
  );
});

export default TreeArea;
