import { BodyHash, ExprIdx, ObligationHash } from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";
import { signal } from "@preact/signals-react";
import _ from "lodash";

export interface HighlightTarget {
  file: Filename;
  bodyIdx: BodyHash;
  exprIdx: ExprIdx;
  hash: ObligationHash;
}

export const highlightedObligation = signal<HighlightTarget | null>(null);
