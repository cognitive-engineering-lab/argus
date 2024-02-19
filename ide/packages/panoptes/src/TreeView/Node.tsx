import {
  Candidate as CandidateTy,
  EvaluationResult,
  Node as NodeTy,
} from "@argus/common/bindings";
import React from "react";

import { IcoAmbiguous, IcoCheck, IcoError, IcoLoop } from "../Icons";
import { PrintGoal, PrintImplHeader } from "../print/print";

export const Result = ({ result }: { result: EvaluationResult }) => {
  return result === "yes" ? (
    <IcoCheck />
  ) : result === "no" ? (
    <IcoError />
  ) : result === "maybe-overflow" ? (
    <IcoLoop />
  ) : (
    <IcoAmbiguous />
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
