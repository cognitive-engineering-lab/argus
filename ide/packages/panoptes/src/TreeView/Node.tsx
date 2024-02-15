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
  switch (candidate.type) {
    case "any":
      return candidate.data;
    case "impl":
      return <PrintImplHeader impl={candidate.data} />;
    case "paramenv":
      throw new Error("paramEnv not implemented");
  }
};

export const Node = ({ node }: { node: NodeTy }) => {
  switch (node.type) {
    case "result":
      return node.data;
    case "goal":
      return (
        <>
          <Result result={node.data.result} />
          <PrintGoal o={node.data} />
        </>
      );
    case "candidate":
      return <Candidate candidate={node.data} />;
  }
};
