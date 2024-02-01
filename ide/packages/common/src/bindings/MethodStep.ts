import { ObligationIdx } from "./ObligationIdx";
import { ReceiverAdjStep } from "./ReceiverAdjStep";

export type MethodStep = {
  recvrTy: ReceiverAdjStep;
  traitPredicates: ObligationIdx[];
};
