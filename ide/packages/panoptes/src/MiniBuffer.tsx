import {
  AllowPathTrim,
  AllowProjectionSubst,
  TyCtxt
} from "@argus/print/context";
import { PrintDefPathFull, PrintTyValue } from "@argus/print/lib";
import { observer } from "mobx-react";
import React from "react";

import { IcoPinned } from "@argus/print/Icons";
import Indented from "@argus/print/Indented";
import { MiniBufferDataStore } from "./signals";
import "./MiniBuffer.css";

const MiniBuffer = observer(() => {
  const data = MiniBufferDataStore.data;
  if (data === undefined) {
    return null;
  }

  const unpinClick = () => MiniBufferDataStore.unpin();
  const heading =
    data.kind === "path" ? (
      <h2>Definition Path</h2>
    ) : data.kind === "projection" ? (
      <h2>Type Projection</h2>
    ) : null;
  const pinned = data.pinned ? <IcoPinned onClick={unpinClick} /> : null;
  const Content = () =>
    data.kind === "path" ? (
      <PrintDefPathFull defPath={data.path} />
    ) : data.kind === "projection" ? (
      <>
        <p>The projected type:</p>
        <Indented>
          <PrintTyValue ty={data.projection} />
        </Indented>
        <p>comes from the definition path:</p>
        <Indented>
          <PrintTyValue ty={data.original} />
        </Indented>
      </>
    ) : null;

  return (
    <>
      <div id="MiniBuffer">
        {pinned}
        {heading}
        <AllowPathTrim.Provider value={false}>
          <AllowProjectionSubst.Provider value={false}>
            <TyCtxt.Provider value={data.ctx}>
              <div className="Data">
                <Content />
              </div>
            </TyCtxt.Provider>
          </AllowProjectionSubst.Provider>
        </AllowPathTrim.Provider>
      </div>
      <div className="spacer">{"\u00A0"}</div>
    </>
  );
});

export default MiniBuffer;
