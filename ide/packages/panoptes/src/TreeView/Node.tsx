import type {
  CandidateIdx,
  EvaluationResult,
  Node as NodeTy,
  ResultIdx
} from "@argus/common/bindings";
import { TreeAppContext } from "@argus/common/context";
import { HoverInfo } from "@argus/print/HoverInfo";
import {
  IcoAmbiguous,
  IcoCheck,
  IcoError,
  IcoEyeClosed,
  IcoLoop
} from "@argus/print/Icons";
import { PrintGoal, PrintImplHeader } from "@argus/print/lib";
import React, { useContext } from "react";

export const ResultRaw = ({ result }: { result: EvaluationResult }) => {
  return result === "yes" ? (
    <HoverInfo Content={() => <span>Proved true</span>}>
      <IcoCheck />
    </HoverInfo>
  ) : result === "no" ? (
    <HoverInfo Content={() => <span>Unsatisfiable</span>}>
      <IcoError />
    </HoverInfo>
  ) : result === "maybe-overflow" ? (
    <HoverInfo
      Content={() => (
        <span>Evaluating this obligation may have caused overflow</span>
      )}
    >
      <IcoLoop />
    </HoverInfo>
  ) : (
    <HoverInfo
      Content={() => (
        <span>Rustc can't determine whether this is true or false</span>
      )}
    >
      <IcoAmbiguous />
    </HoverInfo>
  );
};

export const Result = ({ idx }: { idx: ResultIdx }) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const result = tree.result(idx);
  return <ResultRaw result={result} />;
};

export const Candidate = ({ idx }: { idx: CandidateIdx }) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const candidate = tree.candidate(idx);
  if ("Any" in candidate) {
    return candidate.Any;
  } else if ("Impl" in candidate) {
    const ico = candidate.Impl.is_user_visible ? null : (
      <HoverInfo
        Content={() => <span>This impl block is private to user code.</span>}
      >
        <IcoEyeClosed />
      </HoverInfo>
    );
    return (
      <>
        {ico} <PrintImplHeader impl={candidate.Impl.hd} />
      </>
    );
  } else if ("ParamEnv" in candidate) {
    throw new Error("paramEnv not implemented");
  } else {
    throw new Error("Unknown candidate type", candidate);
  }
};

export const Node = ({ node }: { node: NodeTy }) => {
  const treeInfo = useContext(TreeAppContext.TreeContext)!;
  if ("Result" in node) {
    return (
      <>
        <Result idx={node.Result} /> (end of tree)
      </>
    );
  } else if ("Goal" in node) {
    const goal = treeInfo.goal(node.Goal);
    return (
      <>
        <Result idx={goal.result} />
        <PrintGoal o={goal} />
      </>
    );
  } else if ("Candidate" in node) {
    return <Candidate idx={node.Candidate} />;
  } else {
    throw new Error("Unknown node type", node);
  }
};
