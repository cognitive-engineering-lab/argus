import type { ExprIdx } from "@argus/common/bindings";
import {
  AppContext,
  BodyInfoContext,
  FileContext
} from "@argus/common/context";
import { makeHighlightPosters } from "@argus/common/func";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { useContext } from "react";
import Code from "./Code";
import { ObligationFromIdx } from "./Obligation";
import { CollapsibleElement } from "./TreeView/Directory";
import { HighlightTargetStore } from "./signals";

const Expr = observer(({ idx }: { idx: ExprIdx }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const file = useContext(FileContext)!;
  const expr = bodyInfo.expr(idx);
  if (expr === undefined) return null;

  const messageSystem = useContext(AppContext.MessageSystemContext)!;
  const [addHighlight, removeHighlight] = makeHighlightPosters(
    messageSystem,
    expr.range,
    file
  );

  if (expr.isBody && !bodyInfo.showHidden) {
    return null;
  }

  const visibleObligations = bodyInfo.obligations(idx);
  if (visibleObligations.length === 0) {
    return null;
  }

  const Content = () =>
    _.map(visibleObligations, (oi, i) => (
      <ObligationFromIdx idx={oi} key={i} />
    ));

  const openChildren = idx === HighlightTargetStore.value?.exprIdx;

  // TODO: we should limit the length of the expression snippet or collapse large blocks in some way.
  const header = <Code code={expr.snippet} />;

  return (
    <div onMouseEnter={addHighlight} onMouseLeave={removeHighlight}>
      <CollapsibleElement
        info={header}
        startOpen={openChildren}
        Children={Content}
      />
    </div>
  );
});

export default Expr;
