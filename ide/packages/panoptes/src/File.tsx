import {
  CharRange,
  EvaluationResult,
  Expr,
  ExprIdx,
  MethodLookup,
  MethodLookupIdx,
  Obligation,
  ObligationHash,
  ObligationIdx,
  ObligationsInBody,
  SerializedTree,
} from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";
import { VSCodeButton, VSCodeDivider } from "@vscode/webview-ui-toolkit/react";
import classNames from "classnames";
import _ from "lodash";
import React, {
  Fragment,
  createContext,
  useContext,
  useEffect,
  useState,
} from "react";

import "./File.css";
import { CollapsibleElement } from "./TreeView/Directory";
import TreeApp from "./TreeView/TreeApp";
import {
  PrintBodyName,
  PrintExtensionCandidate,
  PrintObligation,
  PrintTy,
} from "./print/print";
import { WaitingOn } from "./utilities/WaitingOn";
import {
  IcoAmbiguous,
  IcoCheck,
  IcoChevronDown,
  IcoChevronUp,
  IcoError,
  IcoLoop,
} from "./utilities/icons";
import { postToExtension, requestFromExtension } from "./utilities/vscode";

const FileContext = createContext<Filename | undefined>(undefined);
const BodyInfoContext = createContext<BodyInfo | undefined>(undefined);

class BodyInfo {
  constructor(
    readonly oib: ObligationsInBody,
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
    return _.filter(this.oib.exprs[idx].obligations, i => this.notHidden(i));
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
    return o.necessity === "Yes" || this.showHidden;
  }
}

function isObject(x: any): x is object {
  return typeof x === "object" && x !== null;
}

export function obligationCardId(file: Filename, hash: ObligationHash) {
  const name = file.split(/[\\/]/).pop();
  return `obl--${name}-${hash}`;
}

export function errorCardId(
  file: Filename,
  bodyIdx: number,
  errIdx: number,
  errType: "trait" | "ambig"
) {
  const name = file.split(/[\\/]/).pop();
  return `err--${name}-${bodyIdx}-${errType}-${errIdx}`;
}

function makeHighlightPosters(range: CharRange, file: Filename) {
  const addHighlight = () => {
    postToExtension({
      type: "FROM_WEBVIEW",
      file,
      command: "add-highlight",
      range,
    });
  };

  const removeHighlight = () => {
    postToExtension({
      type: "FROM_WEBVIEW",
      file,
      command: "remove-highlight",
      range,
    });
  };

  return [addHighlight, removeHighlight];
}

const NoTreeFound = ({ obligation }: { obligation: Obligation }) => {
  return (
    <div>
      <h3>Couldn't find a proof tree for this obligation</h3>
      <PrintObligation obligation={obligation} />

      <p>This is a bug, please report it!</p>
    </div>
  );
};

const NoObligationFound = ({ hash }: { hash: ObligationHash }) => {
  return (
    <div className="NoInfoError">
      <h3>No obligation found for internal hash {hash}</h3>

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

const ObligationResult = ({ result }: { result: EvaluationResult }) => {
  return result === "yes" ? (
    <IcoCheck />
  ) : result === "no" ? (
    <IcoError />
  ) : result === "maybe-overflow" ? (
    <IcoLoop />
  ) : (
    <IcoAmbiguous />
  );
};

const ObligationCard = ({
  range,
  obligation,
}: {
  range: CharRange;
  obligation: Obligation;
}) => {
  const [isInfoVisible, setIsInfoVisible] = useState(false);
  const file = useContext(FileContext)!;
  const id = obligationCardId(file, obligation.hash);

  const [addHighlight, removeHighlight] = makeHighlightPosters(
    obligation.range,
    file
  );

  const handleClick = () => {
    setIsInfoVisible(!isInfoVisible);
  };

  const cname = classNames("ObligationCard", obligation.result);

  return (
    <div
      id={id}
      className={cname}
      onMouseEnter={addHighlight}
      onMouseLeave={removeHighlight}
    >
      <div className="PrettyObligationArea">
        <span className={classNames("result", obligation.result)}>
          <ObligationResult result={obligation.result} />
        </span>{" "}
        <PrintObligation obligation={obligation} />
      </div>
      <VSCodeButton
        className="ObligationButton"
        appearance="secondary"
        onClick={handleClick}
      >
        {isInfoVisible ? <IcoChevronUp /> : <IcoChevronDown />}
      </VSCodeButton>
      {isInfoVisible && (
        <ObligationTreeWrapper range={range} obligation={obligation} />
      )}
    </div>
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
  return <ObligationResult result={o.result} />;
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
    expr.kind.type !== "methodCall" &&
    bodyInfo.exprObligations(idx).length === 0
  ) {
    return null;
  }

  const content =
    isObject(expr.kind) && expr.kind.type === "methodCall" ? (
      <MethodLookupTable lookup={expr.kind.data} />
    ) : (
      _.map(bodyInfo.exprObligations(idx), (oi, i) => (
        <ObligationFromIdx idx={oi} key={i} />
      ))
    );

  // TODO: we should limit the length of the expression snippet.
  const header = (
    <span>
      Expression <code>{expr.snippet}</code>
    </span>
  );
  return (
    <div onMouseEnter={addHighlight} onMouseLeave={removeHighlight}>
      <CollapsibleElement info={header}>{content}</CollapsibleElement>
    </div>
  );
};

const ObligationBody = ({ bodyInfo }: { bodyInfo: BodyInfo }) => {
  const osib = bodyInfo.oib;
  const errCount = bodyInfo.numErrors;
  const bodyName =
    osib.name === undefined ? (
      "{anon body}"
    ) : (
      <PrintBodyName defPath={osib.name} />
    );
  const header = (
    <span>
      {bodyName}
      {errCount > 0 ? <span style={{ color: "red" }}>({errCount})</span> : ""}
    </span>
  );

  if (!bodyInfo.hasVisibleExprs()) {
    return null;
  }

  return (
    <BodyInfoContext.Provider value={bodyInfo}>
      <CollapsibleElement info={header}>
        {_.map(bodyInfo.exprs, (i, idx) => (
          <InExpr idx={i} key={idx} />
        ))}
      </CollapsibleElement>
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
