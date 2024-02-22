import {
  Candidate as CandidateTy,
  EvaluationResult,
  Node as NodeTy,
} from "@argus/common/bindings";
import React from "react";

import { HoverInfo } from "../HoverInfo";
import { IcoAmbiguous, IcoCheck, IcoError, IcoLoop } from "../Icons";
import { PrintGoal, PrintImplHeader } from "../print/print";

export const Result = ({ result }: { result: EvaluationResult }) => {
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

export const Candidate = ({ candidate }: { candidate: CandidateTy }) => {
  if ("Any" in candidate) {
    return candidate.Any.data;
  } else if ("Impl" in candidate) {
    return <PrintImplHeader impl={candidate.Impl.data} />;
  } else if ("ParamEnv" in candidate) {
    throw new Error("paramEnv not implemented");
  } else {
    throw new Error("Unknown candidate type", candidate);
  }
};

export const Node = ({ node }: { node: NodeTy }) => {
  if ("Result" in node) {
    return <Result result={node.Result.data} />;
  } else if ("Goal" in node) {
    return (
      <>
        <Result result={node.Goal.data.result} />
        <PrintGoal o={node.Goal.data} />
      </>
    );
  } else if ("Candidate" in node) {
    return <Candidate candidate={node.Candidate.data} />;
  } else {
    throw new Error("Unknown node type", node);
  }
};
