import { CharPos } from "./bindings/CharPos";
import { CharRange } from "./bindings/CharRange";
import { FilenameIndex } from "./bindings/FilenameIndex";
import { Node } from "./bindings/Node";
import { Obligation } from "./bindings/Obligation";
import { ObligationsInBody } from "./bindings/ObligationsInBody";
import { SerializedTree } from "./bindings/SerializedTree";
import { TreeTopology } from "./bindings/TreeTopology";

export {
  SerializedTree,
  Node,
  TreeTopology,
  Obligation,
  ObligationsInBody,
  CharPos,
  CharRange,
  FilenameIndex,
};

export type ObligationOutput = ObligationsInBody;
export type TreeOutput = SerializedTree | undefined;
export type ArgusOutputs = ObligationOutput | TreeOutput;
