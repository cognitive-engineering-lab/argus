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
      return <PrintGoal o={node.data} />;
    case "candidate":
      return <Candidate candidate={node.data} />;
  }
};
