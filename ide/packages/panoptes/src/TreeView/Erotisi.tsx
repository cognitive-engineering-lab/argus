import type { TreeInfo } from "@argus/common/TreeInfo";
import type { ProofNodeIdx, SetHeuristic } from "@argus/common/bindings";
import { TreeAppContext } from "@argus/common/context";
import { VSCodeButton } from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React, { useContext, useState } from "react";

import { Node } from "./Node";
import { sortedSubsets } from "./Subsets";

type Target =
  | { kind: "candidate"; node: ProofNodeIdx }
  | { kind: "final"; node: SetHeuristic };

type MaybeProp = Proposition | { reason: string };
interface Proposition {
  target: Target;
  onYes: () => MaybeProp;
  onNo: () => MaybeProp;
  options?: (Target & { kind: "candidate" })[];
}

function formOptions(
  tree: TreeInfo,
  sets: SetHeuristic[],
  roots: ProofNodeIdx[]
): MaybeProp {
  const cansOnly = (s: ProofNodeIdx[]) =>
    _.filter(s, n => "Candidate" in tree.node(n));
  const isParentOf = (
    parent: ProofNodeIdx | undefined,
    child: ProofNodeIdx
  ) => {
    return (
      parent !== undefined &&
      (parent === child ||
        _.includes(cansOnly(tree.pathToRoot(child).path), parent))
    );
  };

  const target = _.head(sets);
  if (!target || target.goals.length === 0) {
    return { reason: "no-target" };
  }

  const highestNode = _.maxBy(target.goals, g => tree.depth(g.idx))!;
  const pathOnlyCandidates = cansOnly(tree.pathFromRoot(highestNode.idx).path);
  // Remove the candidates that are parents of (or equal to) a decided root.
  const path = _.dropWhile(pathOnlyCandidates, n =>
    _.some(roots, r => isParentOf(n, r))
  );

  const targetNode = _.head(path);
  const targetElement: Target =
    targetNode === undefined
      ? { kind: "final", node: target }
      : { kind: "candidate", node: targetNode };

  // If the answer is "yes", then we prune the possibilities to anything
  // that is a child of the target node. Then get the next form possibilities.
  const onYes = () => formOptions(tree, sets, _.concat(roots, targetNode!));
  // If the answer is "no", then we remove the current target set and any
  // other set that is an ancestor of the target node. Then get the next form possibilities.
  const onNo = () => {
    // Any set containing a goal that is an ancestor of the `targetNode` should be removed.
    const newSets = _.filter(sets, s =>
      _.every(s.goals, g => !isParentOf(targetNode, g.idx))
    );
    return formOptions(tree, newSets, roots);
  };

  return {
    target: targetElement,
    onYes,
    onNo
  };
}

const Indented = ({ children }: { children: React.ReactNode }) => (
  <div style={{ marginLeft: "0.5em" }}>{children}</div>
);

const Erotisi = () => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const sets = sortedSubsets(tree.failedSets);
  // If there's only one error we don't need to present questions...
  if (sets.length <= 1) {
    return null;
  }

  const firstForm = formOptions(tree, sets, []);
  const [currentPanel, setCurrentPanel] = useState<MaybeProp>(firstForm);

  const Restart = () => (
    <div>
      <VSCodeButton
        appearance="primary"
        onClick={() => setCurrentPanel(firstForm)}
      >
        Restart
      </VSCodeButton>
    </div>
  );

  // FIXME
  if (typeof currentPanel !== "number" && "reason" in currentPanel) {
    return (
      <>
        <p>No fix aligns with your responses.</p>
        <Restart />
      </>
    );
  }

  const { target, onYes, onNo } = currentPanel;

  const WithYesNo = ({ Q }: { Q: React.FC }) => {
    const yes = () => setCurrentPanel(onYes());
    const no = () => setCurrentPanel(onNo());
    return (
      <div>
        <Q />
        <div style={{ display: "flex", gap: "0.5em", marginTop: "1em" }}>
          <VSCodeButton onClick={no} appearance="secondary" ariaLabel="No">
            No
          </VSCodeButton>
          <VSCodeButton onClick={yes} appearance="primary" ariaLabel="Yes">
            Yes
          </VSCodeButton>
        </div>
      </div>
    );
  };

  const CandidateQ = ({
    node,
    others
  }: {
    node: ProofNodeIdx;
    others?: ProofNodeIdx[];
  }) => {
    const n = tree.node(node);
    const p = tree.parent(node) ?? tree.root;
    const pn = tree.node(p);

    const Question = () => (
      <div>
        <p>Do you expect the predicate:</p>
        <Indented>
          <Node node={pn} />
        </Indented>
        <p>
          to match the <code>impl</code> block
        </p>
        <Indented>
          <Node node={n} />
        </Indented>
      </div>
    );

    return <WithYesNo Q={Question} />;
  };

  const FinalA = ({ set }: { set: SetHeuristic }) => {
    return (
      <>
        <div>
          <p>These failing obligations must be resolved:</p>
          <ul>
            {_.map(set.goals, (g, idx) => (
              <li key={idx}>
                <Node node={tree.node(g.idx)} />
              </li>
            ))}
          </ul>
        </div>
        <Restart />
      </>
    );
  };

  return target.kind === "candidate" ? (
    <CandidateQ node={target.node} />
  ) : target.kind === "final" ? (
    <FinalA set={target.node} />
  ) : null;
};

export default Erotisi;
