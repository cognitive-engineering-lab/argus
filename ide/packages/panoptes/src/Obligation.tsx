import type {
  CharRange,
  ObligationIdx,
  Obligation as ObligationTy,
  SerializedTree
} from "@argus/common/bindings";
import { BodyInfoContext, FileContext } from "@argus/common/context";
import { AppContext } from "@argus/common/context";
import { makeHighlightPosters, obligationCardId } from "@argus/common/func";
import ErrorDiv from "@argus/print/ErrorDiv";
import ReportBugUrl from "@argus/print/ReportBugUrl";
import { PrintObligation } from "@argus/print/lib";
import { observer } from "mobx-react";
import React, {
  useContext,
  useEffect,
  useLayoutEffect,
  useRef,
  useState
} from "react";

import "./Obligation.css";
import { CollapsibleElement } from "./TreeView/Directory";
import { ResultRaw } from "./TreeView/Node";
import TreeApp from "./TreeView/TreeApp";
import { WaitingOn } from "./WaitingOn";
import { HighlightTargetStore } from "./signals";

export const ObligationFromIdx = ({ idx }: { idx: ObligationIdx }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const o = bodyInfo.getObligation(idx);
  return <Obligation range={o.range} obligation={o} />;
};

export const ObligationResultFromIdx = ({ idx }: { idx: ObligationIdx }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const o = bodyInfo.getObligation(idx);
  return <ResultRaw result={o.result} />;
};

const NoTreeFound = ({ obligation }: { obligation: ObligationTy }) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const filename = useContext(FileContext)!;
  return (
    <ErrorDiv>
      <h3>Couldn't find a proof tree for this obligation</h3>
      <PrintObligation obligation={obligation} />

      <p>
        This is a bug,{" "}
        <ReportBugUrl
          error="failed to generate proof tree"
          displayText="click here to report it."
          logText={JSON.stringify({
            filename,
            bodyName: bodyInfo.name,
            obligation
          })}
        />
      </p>
    </ErrorDiv>
  );
};

const ProofTreeWrapper = ({
  range,
  obligation
}: {
  range: CharRange;
  obligation: ObligationTy;
}) => {
  const bodyInfo = useContext(BodyInfoContext)!;
  const [tree, setTree] = useState<SerializedTree | undefined | "loading">(
    "loading"
  );
  const file = useContext(FileContext)!;
  const messageSystem = useContext(AppContext.MessageSystemContext)!;

  useEffect(() => {
    const getData = async () => {
      const tree = await messageSystem.requestData("tree", {
        type: "FROM_WEBVIEW",
        file: file,
        predicate: obligation,
        range: range
      });
      setTree(tree.tree);
    };
    getData();
  }, [file, obligation, range]);

  const content =
    tree === "loading" ? (
      <WaitingOn message="proof tree" />
    ) : tree === undefined ? (
      <NoTreeFound obligation={obligation} />
    ) : (
      <div className="TreeAppObligationWrapper">
        <TreeApp tree={tree} showHidden={bodyInfo.showHidden} />
      </div>
    );

  return content;
};

const Obligation = observer(
  ({ range, obligation }: { range: CharRange; obligation: ObligationTy }) => {
    const file = useContext(FileContext)!;
    const id = obligationCardId(file, obligation.hash);
    const ref = useRef<HTMLDivElement>(null);
    const messageSystem = useContext(AppContext.MessageSystemContext)!;

    const [addHighlight, removeHighlight] = makeHighlightPosters(
      messageSystem,
      obligation.range,
      file
    );

    const isTargetObligation =
      HighlightTargetStore.value?.hash === obligation.hash;

    useLayoutEffect(() => {
      if (isTargetObligation) {
        ref.current?.scrollIntoView({ behavior: "smooth" });
      }
    }, [isTargetObligation]);

    const header = (
      <div
        id={id}
        className="ObligationCard"
        ref={ref}
        onMouseEnter={addHighlight}
        onMouseLeave={removeHighlight}
      >
        <ResultRaw result={obligation.result} />
        <PrintObligation obligation={obligation} />
      </div>
    );

    return (
      <CollapsibleElement
        info={header}
        startOpen={isTargetObligation}
        Children={() => (
          <ProofTreeWrapper range={range} obligation={obligation} />
        )}
      />
    );
  }
);

export default Obligation;
