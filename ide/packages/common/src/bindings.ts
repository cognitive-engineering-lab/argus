import { Candidate } from "./bindings/Candidate";
import { CharPos } from "./bindings/CharPos";
import { CharRange } from "./bindings/CharRange";
import { Expr } from "./bindings/Expr";
import { ExprIdx } from "./bindings/ExprIdx";
import { ExprKind } from "./bindings/ExprKind";
import { FilenameIndex } from "./bindings/FilenameIndex";
import { Goal } from "./bindings/Goal";
import { MethodLookup } from "./bindings/MethodLookup";
import { MethodLookupIdx } from "./bindings/MethodLookupIdx";
import { MethodStep } from "./bindings/MethodStep";
import { Node } from "./bindings/Node";
import { Obligation } from "./bindings/Obligation";
import { ObligationHash } from "./bindings/ObligationHash";
import { ObligationIdx } from "./bindings/ObligationIdx";
import { ObligationsInBody } from "./bindings/ObligationsInBody";
import { SerializedTree } from "./bindings/SerializedTree";
import { TreeTopology } from "./bindings/TreeTopology";

export {
  MethodLookup,
  SerializedTree,
  Node,
  TreeTopology,
  Obligation,
  ObligationsInBody,
  CharPos,
  CharRange,
  FilenameIndex,
  Candidate,
  ObligationHash,
  ObligationIdx,
  Expr,
  ExprKind,
  ExprIdx,
  MethodLookupIdx,
  MethodStep,
  Goal,
};

export type ObligationOutput = ObligationsInBody;
export type TreeOutput = SerializedTree | undefined;
export type ArgusOutputs = ObligationOutput | TreeOutput;
