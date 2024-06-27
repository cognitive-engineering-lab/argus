import _ from "lodash";

import type {
  BodyHash,
  Expr,
  ExprIdx,
  Obligation,
  ObligationHash,
  ObligationIdx,
  ObligationsInBody
} from "./bindings";
import { isHiddenObl } from "./func";

class BodyInfo {
  constructor(
    private readonly oib: ObligationsInBody,
    readonly idx: number,
    public readonly showHidden: boolean
  ) {}

  get hash(): BodyHash {
    return this.oib.hash;
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

  get range() {
    return this.oib.range;
  }

  get start() {
    return this.range.start;
  }

  get end() {
    return this.range.end;
  }

  notHidden(hash: ObligationIdx): boolean {
    const o = this.getObligation(hash);
    if (o === undefined) {
      return false;
    }
    return this.showHidden || isHiddenObl(o);
  }

  hasVisibleExprs() {
    return _.some(this.exprs, idx => this.hasVisibleObligations(idx));
  }

  hasVisibleObligations(idx: ExprIdx) {
    return _.some(this.oib.exprs[idx].obligations, i => this.notHidden(i));
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
}

export default BodyInfo;
