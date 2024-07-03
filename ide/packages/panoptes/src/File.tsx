import BodyInfo from "@argus/common/BodyInfo";
import type { ObligationsInBody } from "@argus/common/bindings";
import {
  AppContext,
  BodyInfoContext,
  FileContext
} from "@argus/common/context";
import type { Filename } from "@argus/common/lib";
import ErrorDiv from "@argus/print/ErrorDiv";
import MonoSpace from "@argus/print/MonoSpace";
import ReportBugUrl from "@argus/print/ReportBugUrl";
import { TyCtxt } from "@argus/print/context";
import { PrintBodyName } from "@argus/print/lib";
import { VSCodeDivider } from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { Fragment, useContext } from "react";

import Expr from "./Expr";
import "./File.css";
import { CollapsibleElement } from "./TreeView/Directory";
import { highlightedObligation } from "./signals";

const FnIndicator = () => <em>ƒ</em>;

const ObligationBody = observer(({ bodyInfo }: { bodyInfo: BodyInfo }) => {
  if (!bodyInfo.hasVisibleExprs()) {
    return null;
  }

  const bodyName =
    bodyInfo.name === undefined ? (
      `{anonymous body}@${bodyInfo.start.line}:${bodyInfo.start.column}`
    ) : (
      <PrintBodyName defPath={bodyInfo.name} />
    );

  const errCount =
    bodyInfo.numErrors > 0 ? (
      <span className="ErrorCount"> ({bodyInfo.numErrors})</span>
    ) : null;

  const header = (
    <>
      <MonoSpace>
        <FnIndicator />
        {"\u00A0"}
        {bodyName}
      </MonoSpace>
      {errCount}
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
  osibs
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
      <TyCtxt.Provider
        value={{
          interner: bodyInfo.tyInterner,
          projections: {}
        }}
      >
        <ObligationBody bodyInfo={bodyInfo} />
      </TyCtxt.Provider>
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
