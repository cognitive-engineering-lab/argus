import { SerializedTree } from "@argus/common/types";
import { TreeTopology } from "@argus/common/types";
import _ from "lodash";
import { observer } from "mobx-react";
import React, {
  MouseEvent,
  MouseEventHandler,
  useCallback,
  useContext,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
// @ts-ignore
import Tree, { Orientation, TreeLinkDatum, TreeNodeDatum } from "react-d3-tree";
import ReactDOM from "react-dom";

import { ActiveContext, TreeContext } from "./Context";
import "./Graph.css";
import { Node } from "./Node";

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

const getEdgeClass =
  (tree: SerializedTree) => (link: TreeLinkDatum, orientation: Orientation) => {
    const sourceIdx = link.source.data.name as number;
    const node = tree.nodes[sourceIdx];

    switch (node.type) {
      case "goal":
        return "edge__goal-to-candidate";
      default:
        return "edge__candidate-to-goal";
    }
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
    const node = treeContext.nodes[idx];
    const label = "NODE CONTENT"; // node.data; // FIXME: bad

    const [width, height] = calculateTextSize(label);

    // const ref = useRef<null | HTMLDivElement>(null);
    // const [width, setWidth] = useState(0);
    // const [height, setHeight] = useState(0);
    // useLayoutEffect(() => {
    //   if (ref != null && ref.current != null) {
    //     setWidth(ref.current.clientWidth);
    //     setHeight(ref.current.clientHeight);
    //   }
    // }, []);

    return (
      <g>
        <rect
          data-sidx={idx}
          x={-width / 2}
          y={-height / 2}
          rx="5"
          ry="5"
          width={width}
          height={height}
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

    // TODO: this royally doesn't work, I'm not sure how to get
    // (or  estimate) the width / height of the node before embedding
    // it into the SVG.
    // <foreignObject x="50%" y="6%" width={width} height={height}>
    //   <div ref={ref} data-xmlns="http://www.w3.org/1999/xhtml">
    //     <NodeContent node={node} />
    //   </div>
    // </foreignObject>
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
        orientation="vertical"
        translate={translate}
        nodeSize={nodeSize}
        renderCustomNodeElement={customRender}
        pathClassFunc={getEdgeClass(tree)}
      />
    </div>
  );
});

export default TreeArea;
