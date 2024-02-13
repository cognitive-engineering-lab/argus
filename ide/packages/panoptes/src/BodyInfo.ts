import {
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

class BodyInfo {
  constructor(
    private readonly oib: ObligationsInBody,
    readonly idx: number,
    readonly viewHiddenObligations: boolean = false
  ) {}

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
    return _.some(this.exprs, idx => this.exprObligations(idx).length > 0);
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

  exprObligations(idx: ExprIdx): ObligationIdx[] {
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
    return o.necessity.type === "yes" || this.showHidden;
  }
}

export default BodyInfo;
