import { CharPos } from "./bindings/CharPos";
import { CharRange } from "./bindings/CharRange";
import { FilenameIndex } from "./bindings/FilenameIndex";
import { Node } from "./bindings/Node";
import { Obligation } from "./bindings/Obligation";
import { SerializedTree } from "./bindings/SerializedTree";
import { TreeTopology } from "./bindings/TreeTopology";


export { SerializedTree, Node, TreeTopology, Obligation, CharPos, CharRange, FilenameIndex };

export type ObligationOutput = { type: "Obligations"; data: Obligation[] };
export type TreeOutput = { type: "Tree"; data: SerializedTree | undefined };
export type ArgusOutputs = ObligationOutput | TreeOutput;