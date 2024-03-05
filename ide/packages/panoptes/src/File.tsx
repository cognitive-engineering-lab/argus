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
import { observer } from "mobx-react";
import React, {
  Fragment,
  createContext,
  useContext,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";

import BodyInfo from "./BodyInfo";
import "./File.css";
import { CollapsibleElement } from "./TreeView/Directory";
import { ResultRaw } from "./TreeView/Node";
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
  const bodyInfo = useContext(BodyInfoContext)!;
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
      <TreeApp tree={tree} showHidden={bodyInfo.viewHiddenObligations} />
    );

  return content;
};

const ObligationCard = observer(
  ({ range, obligation }: { range: CharRange; obligation: Obligation }) => {
    const file = useContext(FileContext)!;
    const id = obligationCardId(file, obligation.hash);
    const ref = useRef<HTMLSpanElement>(null);

    const [addHighlight, removeHighlight] = makeHighlightPosters(
      obligation.range,
      file
    );

    const isTargetObligation =
      highlightedObligation.value?.hash === obligation.hash;
    const className = classNames("ObligationCard", {
      bling: isTargetObligation,
    });

    useLayoutEffect(() => {
      if (highlightedObligation.value?.hash === obligation.hash) {
        ref.current?.scrollIntoView({ behavior: "smooth" });
      }
    }, []);

    const header = (
      <span
        id={id}
        className={className}
        ref={ref}
        onMouseEnter={addHighlight}
        onMouseLeave={removeHighlight}
      >
        <ResultRaw result={obligation.result} />
        <PrintObligation obligation={obligation} />
      </span>
    );

    return (
      <CollapsibleElement
        info={header}
        startOpen={isTargetObligation}
        Children={() => (
          <ObligationTreeWrapper range={range} obligation={obligation} />
        )}
      />
    );
  }
);

const ObligationFromIdx = ({ idx }: { idx: ObligationIdx }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const o = bodyInfo.getObligation(idx);

  return <ObligationCard range={o.range} obligation={o} />;
};

const ObligationResultFromIdx = ({ idx }: { idx: ObligationIdx }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const o = bodyInfo.getObligation(idx);
  return <ResultRaw result={o.result} />;
};

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

const Expr = observer(({ idx }: { idx: ExprIdx }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const file = useContext(FileContext)!;
  const expr = bodyInfo.getExpr(idx);
  const [addHighlight, removeHighlight] = makeHighlightPosters(
    expr.range,
    file
  );

  if (
    !bodyInfo.isErrorMethodCall(expr) &&
    bodyInfo.visibleObligations(idx).length === 0
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

const ObligationBody = observer(({ bodyInfo }: { bodyInfo: BodyInfo }) => {
  if (!bodyInfo.hasVisibleExprs()) {
    return null;
  }

  const errCount = bodyInfo.numErrors;
  const bodyName =
    bodyInfo.name === undefined ? (
      "{anon body}"
    ) : (
      <PrintBodyName defPath={bodyInfo.name} />
    );

  const header = (
    <>
      {bodyName}
      {errCount > 0 ? <span className="ErrorCount">({errCount})</span> : null}
    </>
  );

  const openChildren = bodyInfo.hash === highlightedObligation.value?.bodyIdx;

  return (
    <BodyInfoContext.Provider value={bodyInfo}>
      <CollapsibleElement
        info={header}
        startOpen={openChildren}
        Children={() =>
          _.map(bodyInfo.exprs, (i, idx) => <Expr idx={i} key={idx} />)
        }
      />
    </BodyInfoContext.Provider>
  );
});

const File = ({
  file,
  osibs,
  showHidden = false,
}: {
  file: Filename;
  osibs: ObligationsInBody[];
  showHidden?: boolean;
}) => {
  const bodyInfos = _.map(
    osibs,
    (osib, idx) => new BodyInfo(osib, idx, showHidden)
  );
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
