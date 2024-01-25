import { ObligationHash } from "./ObligationHash";
import { ReceiverAdjStep } from "./ReceiverAdjStep";

export type MethodStep = {
  step: ReceiverAdjStep;
  derefQuery: ObligationHash | undefined;
  relateQuery: ObligationHash | undefined;
  traitPredicates: ObligationHash[];
};
