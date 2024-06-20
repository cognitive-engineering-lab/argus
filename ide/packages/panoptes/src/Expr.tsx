import { ExprIdx } from "@argus/common/bindings";
import {
  AppContext,
  BodyInfoContext,
  FileContext,
} from "@argus/common/context";
import { makeHighlightPosters } from "@argus/common/func";
import classNames from "classnames";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { useContext } from "react";

import Code from "./Code";
import "./File.css";
import { ObligationFromIdx } from "./Obligation";
import { CollapsibleElement } from "./TreeView/Directory";
import { highlightedObligation } from "./signals";

/**
 * Expression-level obligations within a `File`. Expects that
 * the `BodyInfoContext` is available.
 */
const Expr = observer(({ idx }: { idx: ExprIdx }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const file = useContext(FileContext)!;
  const expr = bodyInfo.getExpr(idx);
  const messageSystem = useContext(AppContext.MessageSystemContext)!;
  const [addHighlight, removeHighlight] = makeHighlightPosters(
    messageSystem,
    expr.range,
    file
  );

  if (expr.isBody && !bodyInfo.showHidden) {
    return null;
  }

  const visibleObligations = bodyInfo.visibleObligations(idx);

  if (visibleObligations.length === 0) {
    return null;
  }

  const Content = () =>
    _.map(visibleObligations, (oi, i) => (
      <ObligationFromIdx idx={oi} key={i} />
    ));

  // TODO: we should limit the length of the expression snippet.
  // or at the very least syntax highlight it in some way...
  // I think there should be a better way to represent this information than a blank expr.
  const header = <Code code={expr.snippet} />;

  const openChildren = idx === highlightedObligation.value?.exprIdx;
  // If there is no targeted obligation then we want to highlight
  // the expression level div.
  const className = classNames({
    bling: highlightedObligation.value && !highlightedObligation.value.hash,
  });

  return (
    <div
      className={className}
      onMouseEnter={addHighlight}
      onMouseLeave={removeHighlight}
    >
      <CollapsibleElement
        info={header}
        startOpen={openChildren}
        Children={Content}
      />
    </div>
  );
});

export default Expr;
