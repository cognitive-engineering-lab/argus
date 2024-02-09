import {
  Candidate as CandidateTy,
  Node as NodeTy,
} from "@argus/common/bindings";
import React from "react";

import { PrintGoal, PrintImplHeader, PrintImplHir } from "../print/print";

export const Candidate = ({ candidate }: { candidate: CandidateTy }) => {
  switch (candidate.type) {
    case "any":
      return candidate.data;
    case "implHir":
      return <PrintImplHir impl={candidate.data} />;
    case "implMiddle":
      return <PrintImplHeader impl={candidate.data} />;
    default:
      throw new Error(`Unexpected candidate type ${candidate}`);
  }
};

export const Node = ({ node }: { node: NodeTy }) => {
  switch (node.type) {
    case "result":
      return node.data;
    case "goal":
      return <PrintGoal o={node.data} />;
    case "candidate":
      return <Candidate candidate={node.data} />;
  }
};
