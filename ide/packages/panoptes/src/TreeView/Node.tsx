import { Candidate as CandidateTy, Node as NodeTy } from "@argus/common/bindings";
import React from "react";

import { PrintDefPath } from "../Ty/private/path";
import { PrintGoalPredicate } from "../Ty/private/predicate";
import { PrintTy } from "../Ty/private/ty";

export const Candidate = ({ candidate }: { candidate: CandidateTy }) => {
  switch (candidate.type) {
    case "any":
      return <span>{candidate.data}</span>;
    case "impl":
      const implTrait =
        candidate.traitRef !== undefined ? (
          <PrintDefPath o={candidate.traitRef} />
        ) : (
          "{anon}"
        );
      const candidateTy = <PrintTy o={candidate.ty} />;
      return (
        <span>
          impl {implTrait} for {candidateTy}
        </span>
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
      return <PrintGoalPredicate o={node.data} />;
    case "candidate":
      return <Candidate candidate={node.data} />;
  }
};
