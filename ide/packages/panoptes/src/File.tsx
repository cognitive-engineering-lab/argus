import BodyInfo from "@argus/common/BodyInfo";
import { ObligationsInBody } from "@argus/common/bindings";
import {
  AppContext,
  BodyInfoContext,
  FileContext,
} from "@argus/common/context";
import { Filename } from "@argus/common/lib";
import ErrorDiv from "@argus/print/ErrorDiv";
import ReportBugUrl from "@argus/print/ReportBugUrl";
import { PrintBodyName } from "@argus/print/lib";
import { VSCodeDivider } from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { Fragment, useContext } from "react";

import Expr from "./Expr";
import "./File.css";
import { CollapsibleElement } from "./TreeView/Directory";
import { highlightedObligation } from "./signals";

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
}: {
  file: Filename;
  osibs: ObligationsInBody[];
}) => {
  const showHidden = useContext(AppContext.ShowHiddenObligationsContext);
  const bodyInfos = _.map(
    osibs,
    (osib, idx) => new BodyInfo(osib, idx, showHidden)
  );
  const bodiesWithVisibleExprs = _.filter(bodyInfos, bi =>
    bi.hasVisibleExprs()
  );

  const bodies = _.map(bodiesWithVisibleExprs, (bodyInfo, idx) => (
    <Fragment key={idx}>
      {idx > 0 ? <VSCodeDivider /> : null}
      <ObligationBody bodyInfo={bodyInfo} />
    </Fragment>
  ));

  const noBodiesFound = (
    <ErrorDiv>
      <p>
        Argus didn’t find any “interesting” obligations in this file. If you
        think there should be, please click below to report this as a bug!
      </p>
      <ReportBugUrl
        error={`No informative obligations found in file: ${file}`}
        logText={JSON.stringify({ file, osibs })}
      />
    </ErrorDiv>
  );

  return (
    <FileContext.Provider value={file}>
      {bodies.length > 0 ? bodies : noBodiesFound}
    </FileContext.Provider>
  );
};

export default File;
