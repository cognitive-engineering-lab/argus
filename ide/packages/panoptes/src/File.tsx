import { AmbiguityError, CharRange, Obligation, ObligationHash, ObligationsInBody, SerializedTree, TraitError } from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";
import { VSCodeButton, VSCodeDivider } from "@vscode/webview-ui-toolkit/react";
import classNames from "classnames";
import _ from "lodash";
import React, { RefObject, createContext, useContext, useEffect, useState } from "react";



import "./File.css";
import { CollapsibleElement, ElementPair } from "./TreeView/Directory";
import TreeApp from "./TreeView/TreeApp";
// @ts-ignore
import { PrettyObligation } from "./print/print";
import { PrintTy } from "./print/private/ty";
import { WaitingOn } from "./utilities/WaitingOn";
import {
  IcoAmbiguous,
  IcoCheck,
  IcoChevronDown,
  IcoChevronRight,
  IcoChevronUp,
  IcoError,
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
  obligation(hash: ObligationHash): Obligation | undefined {
    return this.oib.obligations.find(o => o.hash === hash);
  }
  get showHidden(): boolean {
    return this.viewHiddenObligations;
  }
  get numErrors(): number {
    return this.oib.ambiguityErrors.length + this.oib.traitErrors.length;
  }
  get numUnclassified(): number {
    return this.oib.unclassified.length;
  }
  get allObligations(): Obligation[] {
    return _.filter(this.oib.obligations, o => o.isNecessary);
  }
  notHidden(hash: ObligationHash): boolean {
    const o = this.obligation(hash);
    if (o === undefined) {
      return false;
    }
    return o.isNecessary || this.showHidden;
  }
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

const NoTreeFound = ({ obligation }: { obligation: Obligation }) => {
  return (
    <div>
      <h3>Couldn't find a proof tree for this obligation</h3>
      <PrettyObligation obligation={obligation} />

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

  const addHighlight = () => {
    postToExtension({
      type: "FROM_WEBVIEW",
      file: file,
      command: "add-highlight",
      range: obligation.range,
    });
  };

  const removeHighlight = () => {
    postToExtension({
      type: "FROM_WEBVIEW",
      file: file,
      command: "remove-highlight",
      range: obligation.range,
    });
  };

  const handleClick = () => {
    setIsInfoVisible(!isInfoVisible);
  };

  const resultIco =
    obligation.result === "yes" ? (
      <IcoCheck />
    ) : obligation.result === "no" ? (
      <IcoError />
    ) : (
      <IcoAmbiguous />
    );
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
          {resultIco}
        </span>{" "}
        <PrettyObligation obligation={obligation} />
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

const ObligationFromHash = ({ hash }: { hash: ObligationHash }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const o = bodyInfo.obligation(hash);

  if (o === undefined) {
    return <NoObligationFound hash={hash} />;
  }

  return <ObligationCard range={o.range} obligation={o} />;
};

const TraitErrorComponent = ({
  error,
  errIdx,
}: {
  error: TraitError;
  errIdx: number;
}) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const file = useContext(FileContext)!;
  const errorId = errorCardId(file, bodyInfo.idx, errIdx, "trait");
  const visibleCandidates = _.filter(error.candidates, c => bodyInfo.notHidden(c));
  return (
    <div id={errorId}>
      <h4>Trait bound unsatisfied</h4>
      {_.map(visibleCandidates, (candidate, idx) => (
        <ObligationFromHash hash={candidate} key={idx} />
      ))}
    </div>
  );
};

const MethodLookupTable = ({ error }: { error: AmbiguityError }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const table = _.map(error.lookup.table, (step, idx) => {
    const tyComp = (
      <span>
        Receiver type <PrintTy o={step.step.ty} key={idx} />
      </span>
    );

    const obligationsAtStep = _.filter(
      [step.derefQuery, step.relateQuery, ...step.traitPredicates],
      o => o !== undefined && bodyInfo.notHidden(o)
    );

    return (
      <CollapsibleElement info={tyComp} key={idx}>
        <h4>Queries on receiver</h4>
        {_.map(obligationsAtStep, (query, idx) => (
          <ObligationFromHash hash={query!} key={idx} />
        ))}
      </CollapsibleElement>
    );
  });

  const unmarkedToShow = _.filter(error.lookup.unmarked, o => bodyInfo.notHidden(o));
  const unmarked =
    error.lookup.unmarked.length > 0 ? (
      <CollapsibleElement info={<b>Uncategorized obligations</b>}>
        {_.map(unmarkedToShow, (oblHash, idx) => (
          <ObligationFromHash hash={oblHash} key={idx} />
        ))}
      </CollapsibleElement>
    ) : (
      ""
    );

  return (
    <>
      {table}
      {unmarked}
    </>
  );
};

const AmbiguityErrorComponent = ({
  error,
  errIdx,
}: {
  error: AmbiguityError;
  errIdx: number;
}) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const file = useContext(FileContext)!;
  const errorId = errorCardId(file, bodyInfo.idx, errIdx, "ambig");
  return (
    <div id={errorId}>
      <h4>Ambiguous method call</h4>
      <MethodLookupTable error={error} />
    </div>
  );
};

const ObligationBody = ({
  osib,
  idx,
}: {
  osib: ObligationsInBody;
  idx: number;
}) => {
  const bodyName = osib.name;
  const bodyInfo = new BodyInfo(osib, idx);
  const errCount = bodyInfo.numErrors;
  const numUnclassified = bodyInfo.numUnclassified;
  const header = (
    <span>
      Body <code>{bodyName}</code> (
      <span style={{ color: "red" }}>{errCount}</span>)
    </span>
  );

  const unclassifiedFiltered = _.filter(osib.unclassified, o => bodyInfo.notHidden(o));
  const unclassifiedElements =
    numUnclassified == 0 ? (
      ""
    ) : (
      <CollapsibleElement info={<b>Uncategorized obligations</b>}>
        {_.map(unclassifiedFiltered, (oblHash, idx) => (
          <ObligationFromHash hash={oblHash} key={idx} />
        ))}
      </CollapsibleElement>
    );

  return (
    <BodyInfoContext.Provider value={bodyInfo}>
      <CollapsibleElement info={header}>
        {_.map(osib.traitErrors, (error, idx) => (
          <TraitErrorComponent error={error} errIdx={idx} />
        ))}
        {_.map(osib.ambiguityErrors, (error, idx) => (
          <AmbiguityErrorComponent error={error} errIdx={idx} />
        ))}
        {unclassifiedElements}
        <CollapsibleElement info={<b>All obligation in body</b>}>
          {_.map(bodyInfo.allObligations, (obl, idx) => (
            <ObligationCard range={obl.range} obligation={obl} key={idx} />
          ))}
        </CollapsibleElement>
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
  return (
    <FileContext.Provider value={file}>
      {_.map(osibs, (osib, idx) => (
        <>
          <VSCodeDivider />
          <ObligationBody osib={osib} idx={idx} key={idx} />
        </>
      ))}
    </FileContext.Provider>
  );
};

export default File;