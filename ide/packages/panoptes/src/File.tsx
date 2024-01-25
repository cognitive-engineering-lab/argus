import {
  AmbiguityError,
  CharRange,
  Obligation,
  ObligationHash,
  ObligationsInBody,
  SerializedTree,
  TraitError,
} from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";
import { VSCodeButton, VSCodeDivider } from "@vscode/webview-ui-toolkit/react";
import classNames from "classnames";
import _ from "lodash";
import React, {
  RefObject,
  createContext,
  useContext,
  useEffect,
  useState,
} from "react";

import "./File.css";
import { CollapsibleElement, ElementPair } from "./TreeView/Directory";
import TreeApp from "./TreeView/TreeApp";
// @ts-ignore
import { PrettyObligation } from "./Ty/print";
import { PrintTy } from "./Ty/private/ty";
import { WaitingOn } from "./utilities/WaitingOn";
import {
  IcoAmbiguous,
  IcoCheck,
  IcoChevronDown,
  IcoChevronUp,
  IcoError,
  IcoTriangleDown,
  IcoTriangleRight,
} from "./utilities/icons";
import { postToExtension, requestFromExtension } from "./utilities/vscode";

const FileContext = createContext<Filename | undefined>(undefined);
const BodyInfoContext = createContext<BodyInfo | undefined>(undefined);

class BodyInfo {
  constructor(readonly oib: ObligationsInBody, readonly idx: number) {}
  obligation(hash: ObligationHash): Obligation | undefined {
    return this.oib.obligations.find(o => o.hash === hash);
  }
  get numErrors(): number {
    return this.oib.ambiguityErrors.length + this.oib.traitErrors.length;
  }
  get numUnclassified(): number {
    return this.oib.unclassified.length;
  }
  get allObligations(): Obligation[] {
    return this.oib.obligations;
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

  return <>{content}</>;
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
  return (
    <div id={errorId}>
      <h4>Trait bound unsatisfied</h4>
      {_.map(error.candidates, (candidate, idx) => {
        return <ObligationFromHash hash={candidate} key={idx} />;
      })}
    </div>
  );
};

const MethodLookupTable = ({ error }: { error: AmbiguityError }) => {
  const arrows: ElementPair = [<IcoTriangleDown />, <IcoTriangleRight />];
  const table = _.map(error.lookup.table, (step, idx) => {
    const tyComp = (
      <span>
        Receiver type <PrintTy o={step.step.ty} key={idx} />
      </span>
    );
    return (
      <CollapsibleElement info={tyComp} icos={arrows} key={idx}>
        {step.derefQuery === undefined ? (
          ""
        ) : (
          <>
            <h4>Testing deref</h4>
            <ObligationFromHash hash={step.derefQuery} />
          </>
        )}
        {step.relateQuery === undefined ? (
          ""
        ) : (
          <>
            <h4>
              Relating to <code>Deref::Target</code>
            </h4>
            <ObligationFromHash hash={step.relateQuery} />
          </>
        )}
        {step.traitPredicates.length === 0 ? (
          ""
        ) : (
          <>
            <h4>Trait queries</h4>
            {_.map(step.traitPredicates, (query, idx) => {
              return <ObligationFromHash hash={query} key={idx} />;
            })}
          </>
        )}
      </CollapsibleElement>
    );
  });

  const unmarked =
    error.lookup.unmarked.length > 0 ? (
      <CollapsibleElement info={<b>Uncategorized obligations</b>} icos={arrows}>
        {_.map(error.lookup.unmarked, (oblHash, idx) => {
          return <ObligationFromHash hash={oblHash} key={idx} />;
        })}
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
  const arrows: ElementPair = [<IcoTriangleDown />, <IcoTriangleRight />];
  const errCount = bodyInfo.numErrors;
  const numUnclassified = bodyInfo.numUnclassified;
  const header = (
    <span>
      Body <code>{bodyName}</code> (
      <span style={{ color: "red" }}>{errCount}</span>)
    </span>
  );

  return (
    <BodyInfoContext.Provider value={bodyInfo}>
      <CollapsibleElement info={header} icos={arrows}>
        {_.map(osib.traitErrors, (error, idx) => {
          return <TraitErrorComponent error={error} errIdx={idx} />;
        })}
        {_.map(osib.ambiguityErrors, (error, idx) => {
          return <AmbiguityErrorComponent error={error} errIdx={idx} />;
        })}
        {numUnclassified == 0 ? (
          ""
        ) : (
          <CollapsibleElement
            info={<b>Uncategorized obligations</b>}
            icos={arrows}
          >
            {_.map(osib.unclassified, (oblHash, idx) => {
              return <ObligationFromHash hash={oblHash} key={idx} />;
            })}
          </CollapsibleElement>
        )}
        <h3>All obligation in body</h3>
        {_.map(bodyInfo.allObligations, (obl, idx) => {
          return (
            <ObligationCard range={obl.range} obligation={obl} key={idx} />
          );
        })}
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
      {_.map(osibs, (osib, idx) => {
        return (
          <>
            <VSCodeDivider />
            <ObligationBody osib={osib} idx={idx} key={idx} />
          </>
        );
      })}
    </FileContext.Provider>
  );
};

export default File;
