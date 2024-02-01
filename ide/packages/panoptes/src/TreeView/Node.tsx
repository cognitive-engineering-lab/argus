import {
  Candidate as CandidateTy,
  Node as NodeTy,
} from "@argus/common/bindings";
import React from "react";

import { PrintGoal, PrintImpl } from "../print/print";

export const Candidate = ({ candidate }: { candidate: CandidateTy }) => {
  switch (candidate.type) {
    case "any":
      return <span>{candidate.data}</span>;
    case "impl":
      return candidate.data === undefined ? (
        <span>{candidate.fallback}</span>
      ) : (
        <PrintImpl impl={candidate.data} />
      );
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
