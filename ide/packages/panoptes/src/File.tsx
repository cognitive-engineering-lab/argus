import BodyInfo from "@argus/common/BodyInfo";
import type { ObligationsInBody } from "@argus/common/bindings";
import {
  AppContext,
  BodyInfoContext,
  FileContext
} from "@argus/common/context";
import type { Filename } from "@argus/common/lib";
import { ErrorDiv, InfoDiv } from "@argus/print/Info";
import MonoSpace from "@argus/print/MonoSpace";
import ReportBugUrl from "@argus/print/ReportBugUrl";
import { TyCtxt } from "@argus/print/context";
import { PrintBodyName } from "@argus/print/lib";
import { nbsp } from "@argus/print/syntax";
import { VSCodeBadge, VSCodeDivider } from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import { observer } from "mobx-react";
import React, { Fragment, useContext } from "react";

import Expr from "./Expr";
import "./File.css";
import { CollapsibleElement } from "./TreeView/Directory";
import { HighlightTargetStore } from "./signals";

const fnIndicator = <em>ƒ</em>;

const ObligationBody = observer(({ bodyInfo }: { bodyInfo: BodyInfo }) => {
  if (!bodyInfo.hasVisibleExprs()) return null;

  const bodyName =
    bodyInfo.name === undefined ? (
      `{anonymous body}@${bodyInfo.start.line}:${bodyInfo.start.column}`
    ) : (
      <PrintBodyName defPath={bodyInfo.name} />
    );

  const errCount =
    bodyInfo.numErrors > 0 ? (
      <>
        {nbsp}
        <VSCodeBadge>{bodyInfo.numErrors}</VSCodeBadge>
      </>
    ) : null;

  const header = (
    <>
      <MonoSpace idx={bodyInfo.hash}>
        {fnIndicator}
        {"\u00A0"}
        {bodyName}
      </MonoSpace>
      {errCount}
    </>
  );

  const Kids = () =>
    _.map(bodyInfo.exprs(), (i, idx) => <Expr idx={i} key={idx} />);

  const openChildren = bodyInfo.hash === HighlightTargetStore.value?.bodyIdx;

  return (
    <BodyInfoContext.Provider value={bodyInfo}>
      <CollapsibleElement
        info={header}
        startOpen={openChildren}
        Children={Kids}
      />
    </BodyInfoContext.Provider>
  );
});

export interface FileProps {
  file: Filename;
  osibs: ObligationsInBody[];
}

const File = ({ file, osibs }: FileProps) => {
  const settings = useContext(AppContext.SettingsContext);
  const bodyInfos = _.map(
    osibs,
    osib => new BodyInfo(osib, settings["show-hidden-obligations"])
  );

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

  const fileTypecks = (
    <InfoDiv>
      <p>
        Argus thinks this file type checks! You may close the Argus inspector
        panel. If you think it shouldn’t, please click below to report this as a
        bug!
      </p>
      <ReportBugUrl
        error={`File typecks but shouldn't: ${file}`}
        logText={JSON.stringify({ file, osibs })}
      />
    </InfoDiv>
  );

  const bodiesWithVisibleExprs = _.filter(bodyInfos, bi =>
    bi.hasVisibleExprs()
  );

  if (bodiesWithVisibleExprs.length === 0) {
    if (_.some(bodyInfos, bi => bi.isTainted)) {
      return noBodiesFound;
    } else {
      return fileTypecks;
    }
  }

  return (
    <FileContext.Provider value={file}>
      {_.map(bodiesWithVisibleExprs, (bodyInfo, idx) => (
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
      ))}
    </FileContext.Provider>
  );
};

export default File;
