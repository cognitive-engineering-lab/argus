import {
  ExprIdx,
  MethodLookupIdx,
  ObligationIdx,
} from "@argus/common/bindings";
import {
  AppContext,
  BodyInfoContext,
  FileContext,
} from "@argus/common/context";
import { isObject, makeHighlightPosters } from "@argus/common/func";
import { PrintExtensionCandidate, PrintTy } from "@argus/print/lib";
import classNames from "classnames";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { useContext, useState } from "react";

import "./File.css";
import { ObligationFromIdx, ObligationResultFromIdx } from "./Obligation";
import { CollapsibleElement } from "./TreeView/Directory";
import { highlightedObligation } from "./signals";

const MethodLookupTable = ({ lookup }: { lookup: MethodLookupIdx }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const lookupInfo = bodyInfo.getMethodLookup(lookup);
  const numCans = lookupInfo.candidates.data.length ?? 0;

  const [hoveredObligation, setActiveObligation] = useState<
    ObligationIdx | undefined
  >(undefined);

  const [clickedObligation, setClickedObligation] = useState<
    [ObligationIdx, React.ReactElement] | null
  >(null);

  const onTDHover = (idx: ObligationIdx) => () => setActiveObligation(idx);
  const onTableMouseExit = () => setActiveObligation(undefined);
  const onClick = (idx: ObligationIdx) => () =>
    setClickedObligation([idx, <ObligationFromIdx idx={idx} />]);

  const headingRow = (
    <tr>
      <th>Receiver Ty</th>
      {_.map(_.range(numCans), (i, idx) => (
        <th>
          <PrintExtensionCandidate
            idx={i}
            candidates={lookupInfo.candidates}
            key={idx}
          />
        </th>
      ))}
    </tr>
  );

  // TODO: the ObligationResult should be interactive, showing the predicate
  // on hover, and on click should extand an info box with the TreeApp.
  const bodyRows = _.map(lookupInfo.table, (step, idx) => (
    <tr key={idx}>
      <td>
        <PrintTy ty={step.recvrTy.ty} />
      </td>
      {_.map(step.traitPredicates, (queryIdx, idx) => (
        <td
          key={idx}
          className={classNames("with-result", {
            active: queryIdx === clickedObligation?.[0],
          })}
          onMouseEnter={onTDHover(queryIdx)}
          onClick={onClick(queryIdx)}
        >
          <ObligationResultFromIdx idx={queryIdx} />
        </td>
      ))}
    </tr>
  ));

  return (
    <>
      <table className="MethodCallTable" onMouseLeave={onTableMouseExit}>
        {headingRow}
        {bodyRows}
      </table>
      {hoveredObligation !== undefined ? (
        <ObligationFromIdx idx={hoveredObligation} />
      ) : (
        clickedObligation?.[1]
      )}
    </>
  );
};

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

  if (
    (expr.isBody && !bodyInfo.showHidden) ||
    (!bodyInfo.isErrorMethodCall(expr) &&
      bodyInfo.visibleObligations(idx).length === 0)
  ) {
    return null;
  }

  const Content = () =>
    isObject(expr.kind) && "MethodCall" in expr.kind ? (
      <MethodLookupTable lookup={expr.kind.MethodCall.data} />
    ) : (
      _.map(bodyInfo.visibleObligations(idx), (oi, i) => (
        <ObligationFromIdx idx={oi} key={i} />
      ))
    );

  // TODO: we should limit the length of the expression snippet.
  // or at the very least syntax highlight it in some way...
  const header = <pre>{expr.snippet}</pre>;

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
