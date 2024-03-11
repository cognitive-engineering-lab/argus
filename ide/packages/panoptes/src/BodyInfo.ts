import {
  BodyHash,
  Expr,
  ExprIdx,
  MethodLookup,
  MethodLookupIdx,
  Obligation,
  ObligationHash,
  ObligationIdx,
  ObligationsInBody,
} from "@argus/common/bindings";
import _ from "lodash";

import { isObject } from "./utilities/func";

class BodyInfo {
  constructor(
    private readonly oib: ObligationsInBody,
    readonly idx: number,
    readonly viewHiddenObligations: boolean = false
  ) {}

  get hash(): BodyHash {
    return this.oib.hash;
  }

  get showHidden(): boolean {
    return this.viewHiddenObligations;
  }

  get numErrors(): number {
    return this.oib.ambiguityErrors.length + this.oib.traitErrors.length;
  }

  get exprs(): ExprIdx[] {
    return _.map(this.oib.exprs, (_, idx) => idx);
  }

  get name() {
    return this.oib.name;
  }

  hasVisibleExprs() {
    return _.some(this.exprs, idx => this.visibleObligations(idx).length > 0);
  }

  byHash(hash: ObligationHash): Obligation | undefined {
    return this.oib.obligations.find(o => o.hash === hash);
  }

  getObligation(idx: ObligationIdx): Obligation {
    return this.oib.obligations[idx];
  }

  getExpr(idx: ExprIdx): Expr {
    return this.oib.exprs[idx];
  }

  isErrorMethodCall(expr: Expr): boolean {
    if (!(isObject(expr.kind) && "MethodCall" in expr.kind)) {
      return false;
    }

    if (expr.kind.MethodCall.errorRecvr) {
      return true;
    }

    const lookup = this.getMethodLookup(expr.kind.MethodCall.data);

    // This is an error method call if there doesn't exist an entry with a result "yes".
    return !_.some(lookup.table, step =>
      _.some(
        step.traitPredicates,
        idx => this.getObligation(idx).result === "yes"
      )
    );
  }

  visibleObligations(idx: ExprIdx): ObligationIdx[] {
    const filtered = _.filter(this.oib.exprs[idx].obligations, i =>
      this.notHidden(i)
    );
    const sorted = _.sortBy(filtered, i => {
      switch (this.getObligation(i).result) {
        case "no":
          return 0;
        case "yes":
          return 2;
        default:
          return 1;
      }
    });
    return sorted;
  }

  getMethodLookup(idx: MethodLookupIdx): MethodLookup {
    console.debug(
      "Method lookups: ",
      this.oib.methodLookups.length,
      idx,
      this.oib.methodLookups
    );
    return this.oib.methodLookups[idx];
  }

  notHidden(hash: ObligationIdx): boolean {
    const o = this.getObligation(hash);
    if (o === undefined) {
      return false;
    }
    return (
      this.showHidden || o.necessity === "Yes"
      // TODO: this includes obligations like `(): TRAIT` which seem to happen
      // way too frequently and even on error shouldn't be shown.
      // || (o.necessity === "OnError" && o.result === "no")
    );
  }
}

export default BodyInfo;
