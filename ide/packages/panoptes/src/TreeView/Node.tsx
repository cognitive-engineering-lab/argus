import {
  Candidate as CandidateTy,
  Node as NodeTy,
} from "@argus/common/bindings";
import React from "react";

import { PrintImpl } from "../print/private/hir";
import { PrintDefPath } from "../print/private/path";
import { PrintGoalPredicate } from "../print/private/predicate";
import { PrintTy } from "../print/private/ty";

export const Candidate = ({ candidate }: { candidate: CandidateTy }) => {
  switch (candidate.type) {
    case "any":
      return <span>{candidate.data}</span>;
    case "impl":
      return <PrintImpl impl={candidate.data} />;
    default:
      throw new Error(`Unexpected candidate type ${candidate}`);
  }
};

export const Node = ({ node }: { node: NodeTy }) => {
  switch (node.type) {
    case "result":
      return node.data;
    case "goal":
      return <PrintGoalPredicate o={node.data} />;
    case "candidate":
      return <Candidate candidate={node.data} />;
  }
};
