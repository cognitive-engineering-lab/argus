import {
  CharRange,
  ExprIdx,
  MethodLookupIdx,
  Obligation,
  ObligationIdx,
  ObligationsInBody,
  SerializedTree,
} from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";
import { VSCodeDivider } from "@vscode/webview-ui-toolkit/react";
import classNames from "classnames";
import _ from "lodash";
import React, {
  Fragment,
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";

import BodyInfo from "./BodyInfo";
import "./File.css";
import { CollapsibleElement } from "./TreeView/Directory";
import { Result } from "./TreeView/Node";
import TreeApp from "./TreeView/TreeApp";
import { WaitingOn } from "./WaitingOn";
import {
  PrintBodyName,
  PrintExtensionCandidate,
  PrintObligation,
  PrintTy,
} from "./print/print";
import { highlightedObligation } from "./signals";
import {
  isObject,
  makeHighlightPosters,
  obligationCardId,
} from "./utilities/func";
import { requestFromExtension } from "./utilities/vscode";

const FileContext = createContext<Filename | undefined>(undefined);
const BodyInfoContext = createContext<BodyInfo | undefined>(undefined);

const NoTreeFound = ({ obligation }: { obligation: Obligation }) => {
  return (
    <div>
      <h3>Couldn't find a proof tree for this obligation</h3>
      <PrintObligation obligation={obligation} />

      <p>This is a bug, please report it!</p>
    </div>
  );
};

const ObligationTreeWrapper = ({
  range,
  obligation,
}: {
  range: CharRange;
  obligation: Obligation;
}) => {
  const [tree, setTree] = useState<SerializedTree | undefined | "loading">(
    "loading"
  );
  const file = useContext(FileContext)!;

  useEffect(() => {
    const getData = async () => {
      const tree = await requestFromExtension<"tree">({
        type: "FROM_WEBVIEW",
        file: file,
        command: "tree",
        predicate: obligation,
        range: range,
      });
      setTree(tree.tree);
    };
    getData();
  }, []);

  const content =
    tree === "loading" ? (
      <WaitingOn message="proof tree" />
    ) : tree === undefined ? (
      <NoTreeFound obligation={obligation} />
    ) : (
      <TreeApp tree={tree} />
    );

  return content;
};

const ObligationCard = ({
  range,
  obligation,
}: {
  range: CharRange;
  obligation: Obligation;
}) => {
  const file = useContext(FileContext)!;
  const id = obligationCardId(file, obligation.hash);
  const ref = useRef<HTMLSpanElement>(null);

  const [addHighlight, removeHighlight] = makeHighlightPosters(
    obligation.range,
    file
  );

  useEffect(() => {
    if (highlightedObligation.value?.hash === obligation.hash) {
      ref.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, []);

  const className = classNames("result", {
    bling: highlightedObligation.value?.hash === obligation.hash,
  });

  const header = (
    <span
      id={id}
      ref={ref}
      onMouseEnter={addHighlight}
      onMouseLeave={removeHighlight}
    >
      <span className={className}>
        <Result result={obligation.result} />
      </span>{" "}
      <PrintObligation obligation={obligation} />
    </span>
  );

  return (
    <CollapsibleElement
      info={header}
      Children={() => (
        <ObligationTreeWrapper range={range} obligation={obligation} />
      )}
    />
  );
};

const ObligationFromIdx = ({ idx }: { idx: ObligationIdx }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const o = bodyInfo.getObligation(idx);

  return <ObligationCard range={o.range} obligation={o} />;
};

const ObligationResultFromIdx = ({ idx }: { idx: ObligationIdx }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const o = bodyInfo.getObligation(idx);
  return <Result result={o.result} />;
};

const MethodLookupTable = ({ lookup }: { lookup: MethodLookupIdx }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const lookupInfo = bodyInfo.getMethodLookup(lookup);
  const numCans = lookupInfo.candidates.data.length ?? 0;

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
        <td key={idx}>
          <ObligationResultFromIdx idx={queryIdx} />
        </td>
      ))}
    </tr>
  ));

  return (
    <table>
      {headingRow}
      {bodyRows}
    </table>
  );
};

// NOTE: don't access the expression obligations directly, use the BodyInfo
// to get the obligations that are currently visible.
const InExpr = ({ idx }: { idx: ExprIdx }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const file = useContext(FileContext)!;
  const expr = bodyInfo.getExpr(idx);
  const [addHighlight, removeHighlight] = makeHighlightPosters(
    expr.range,
    file
  );

  if (
    isObject(expr.kind) &&
    expr.kind.type !== "methodcall" &&
    bodyInfo.exprObligations(idx).length === 0
  ) {
    return null;
  }

  const content =
    isObject(expr.kind) && expr.kind.type === "methodcall" ? (
      <MethodLookupTable lookup={expr.kind.data} />
    ) : (
      _.map(bodyInfo.exprObligations(idx), (oi, i) => (
        <ObligationFromIdx idx={oi} key={i} />
      ))
    );

  const openChildren = idx === highlightedObligation.value?.exprIdx;

  // TODO: we should limit the length of the expression snippet.
  const header = <code>{expr.snippet}</code>;
  return (
    <div onMouseEnter={addHighlight} onMouseLeave={removeHighlight}>
      <CollapsibleElement
        info={header}
        startOpen={openChildren}
        Children={() => content}
      />
    </div>
  );
};

const ObligationBody = ({ bodyInfo }: { bodyInfo: BodyInfo }) => {
  const errCount = bodyInfo.numErrors;
  const bodyName =
    bodyInfo.name === undefined ? (
      "{anon body}"
    ) : (
      <PrintBodyName defPath={bodyInfo.name} />
    );
  const header = (
    <span>
      {bodyName}
      {errCount > 0 ? <span className="ErrorCount">({errCount})</span> : null}
    </span>
  );

  if (!bodyInfo.hasVisibleExprs()) {
    return null;
  }

  const openChildren = bodyInfo.hash === highlightedObligation.value?.bodyIdx;

  return (
    <BodyInfoContext.Provider value={bodyInfo}>
      <CollapsibleElement
        info={header}
        startOpen={openChildren}
        Children={() =>
          _.map(bodyInfo.exprs, (i, idx) => <InExpr idx={i} key={idx} />)
        }
      />
    </BodyInfoContext.Provider>
  );
};

const File = ({
  file,
  osibs,
}: {
  file: Filename;
  osibs: ObligationsInBody[];
}) => {
  const bodyInfos = _.map(osibs, (osib, idx) => new BodyInfo(osib, idx));
  const bodiesWithVisibleExprs = _.filter(bodyInfos, bi =>
    bi.hasVisibleExprs()
  );
  return (
    <FileContext.Provider value={file}>
      {_.map(bodiesWithVisibleExprs, (bodyInfo, idx) => (
        <Fragment key={idx}>
          {idx > 0 ? <VSCodeDivider /> : null}
          <ObligationBody bodyInfo={bodyInfo} />
        </Fragment>
      ))}
    </FileContext.Provider>
  );
};

export default File;
