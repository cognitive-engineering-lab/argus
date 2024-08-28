import _ from "lodash";

import type {
  BodyHash,
  Expr,
  ExprIdx,
  Obligation,
  ObligationIdx,
  ObligationsInBody
} from "./bindings";
import { isVisibleObligation } from "./func";

class BodyInfo {
  private existsImportantFailure;

  constructor(
    private readonly oib: ObligationsInBody,
    public readonly showHidden: boolean
  ) {
    // An important failure is a *necessary* and *failing* obligation. We say that
    // there exists an important failure if any of the expressions has an obligation
    // that meets this criteria.
    this.existsImportantFailure = false;
    this.existsImportantFailure = _.some(this.exprs(), eidx =>
      _.some(this.obligations(eidx), oidx => {
        const o = this.obligation(oidx)!;
        return o.result === "no" && o.necessity === "Yes";
      })
    );
  }

  get hash(): BodyHash {
    return this.oib.hash;
  }

  get numErrors(): number {
    return this.oib.ambiguityErrors.length + this.oib.traitErrors.length;
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

  get tyInterner() {
    return this.oib.tys;
  }

  exprs(): ExprIdx[] {
    return _.range(0, this.oib.exprs.length);
  }

  expr(idx: ExprIdx): Expr | undefined {
    return this.oib.exprs[idx];
  }

  private visibleObligations(idx: ExprIdx): ObligationIdx[] {
    return _.filter(this.oib.exprs[idx].obligations, i => this.isVisible(i));
  }

  obligations(idx: ExprIdx): ObligationIdx[] {
    return _.sortBy(this.visibleObligations(idx), i => {
      switch (this.obligation(i)!.result) {
        case "no":
          return 0;
        case "yes":
          return 2;
        default:
          return 1;
      }
    });
  }

  obligation(idx: ObligationIdx): Obligation | undefined {
    return this.oib.obligations[idx];
  }

  // Does this body have any expressions that have visible obligations?
  hasVisibleExprs() {
    return _.some(this.exprs(), idx => this.hasVisibleObligations(idx));
  }

  // Does the given expression have any visible obligations?
  hasVisibleObligations(idx: ExprIdx) {
    return this.visibleObligations(idx).length > 0;
  }

  // Is the given obligation visible?
  isVisible(idx: ObligationIdx) {
    const o = this.obligation(idx);
    if (o === undefined) return false;

    return (
      this.showHidden ||
      isVisibleObligation(
        o,
        // HACK: If there is a failing obligation, we filter ambiguities. This is
        // a short workaround for a backend incompleteness. We can't filter obligations
        // that get resolved in a second round of trait solving, this leaves Argus with
        // more "failures" than rustc shows.
        this.existsImportantFailure
      )
    );
  }
}

export default BodyInfo;
