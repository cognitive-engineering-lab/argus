import { TreeInfo } from "@argus/common/TreeInfo";
import { ProofNodeIdx, TreeTopology } from "@argus/common/bindings";
import { TreeAppContext } from "@argus/common/context";
import _ from "lodash";
import React, {
  MouseEventHandler,
  useCallback,
  useContext,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
// @ts-ignore
import Tree, { Orientation, TreeLinkDatum, TreeNodeDatum } from "react-d3-tree";

import "./Graph.css";
import { Node, Result } from "./Node";

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

const getEdgeClass =
  (tree: TreeInfo) => (link: TreeLinkDatum, _orientation: Orientation) => {
    const sourceIdx = link.source.data.name as number;
    const node = tree.node(sourceIdx);
    return "Goal" in node
      ? "edge__goal-to-candidate"
      : "edge__candidate-to-goal";
  };

const TreeNode = ({
  nodeDatum,
  toggleNode,
  hoverNode,
}: {
  nodeDatum: TreeNodeDatum;
  toggleNode: MouseEventHandler<SVGRectElement>;
  hoverNode: MouseEventHandler<SVGRectElement>;
}) => {
  const treeInfo = useContext(TreeAppContext.TreeContext)!;
  const ref = useRef<HTMLDivElement>(null);
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });
  const padding = 10;

  const idx = nodeDatum.name as number;
  const node = treeInfo.node(idx);

  useLayoutEffect(() => {
    if (ref.current) {
      setDimensions({
        width: ref.current.offsetWidth,
        height: ref.current.offsetHeight,
      });
    }
  }, []);

  const RectangleNode = () => (
    <rect
      data-sidx={idx}
      x={-dimensions.width / 2}
      y={-dimensions.height / 2}
      width={dimensions.width + padding}
      height={dimensions.height + padding}
      rx="3"
      ry="3"
      onClick={toggleNode}
      onMouseEnter={hoverNode}
    />
  );

  const CircleNode = () => (
    <circle cy={padding / 2} r={(dimensions.width + padding) / 2} />
  );

  const Shape = "Result" in node ? CircleNode : RectangleNode;

  // data-xmlns="http://www.w3.org/1999/xhtml"
  return (
    <g>
      <Shape />
      <foreignObject
        x={-dimensions.width / 2}
        y={-dimensions.height / 2}
        width="100%"
        height="100%"
      >
        <span ref={ref} className="foreign-wrapper">
          {"Result" in node ? (
            <Result idx={node.Result} />
          ) : (
            <Node node={node} />
          )}
        </span>
      </foreignObject>
    </g>
  );
};

const topologyToTreeData = (
  topology: TreeTopology,
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

const Graph = ({ root }: { root: ProofNodeIdx }) => {
  const treeInfo = useContext(TreeAppContext.TreeContext)!;
  const [translate, containerRef] = useCenteredTree();

  const topology = treeInfo.topology;
  const data = topologyToTreeData(topology, root);

  const customRender = (rd3tProps: any) => <TreeNode {...rd3tProps} />;

  const nodeSize = { x: 250, y: 100 };

  return (
    <div className="TreeArea" ref={containerRef}>
      <Tree
        data={data}
        orientation="horizontal"
        translate={translate}
        nodeSize={nodeSize}
        renderCustomNodeElement={customRender}
        pathClassFunc={getEdgeClass(treeInfo)}
        separation={{ siblings: 1, nonSiblings: 1 }}
      />
    </div>
  );
};

export default Graph;
