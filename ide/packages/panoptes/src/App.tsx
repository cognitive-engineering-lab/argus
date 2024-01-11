import { ObligationHash, ObligationsInBody } from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";
import _ from "lodash";
import React, { RefObject, useEffect, useState } from "react";

import File, { ObligationHookContext } from "./File";
import Workspace from "./Workspace";
import { WaitingOn } from "./utilities/WaitingOn";
import { requestFromExtension } from "./utilities/vscode";

const OpenFile = ({ filename }: { filename: Filename }) => {
  const [obligations, setObligations] = useState<
    ObligationsInBody[] | undefined
  >(undefined);

  // FIXME: is this right, we only want o load things once.
  useEffect(() => {
    const getData = async () => {
      const obligations = await requestFromExtension<"obligations">({
        type: "FROM_WEBVIEW",
        command: "obligations",
        file: filename,
      });
      setObligations(obligations.obligations);
    };
    getData();
  }, []);

  return (
    <div>
      {obligations === undefined ? (
        <WaitingOn message="obligations" />
      ) : (
        <File file={filename} osibs={obligations!} />
      )}
    </div>
  );
};

const App = ({ initialFiles }: { initialFiles: Filename[] }) => {
  const [openFiles, setOpenFiles] = useState<[Filename, React.ReactElement][]>(
    _.map(initialFiles, filename => {
      return [filename, <OpenFile filename={filename} />];
    })
  );

  const [obligations, setObligations] = useState<
    [Filename, ObligationHash, RefObject<HTMLDivElement>][]
  >([]);
  const addRefToList = (
    file: Filename,
    hash: ObligationHash,
    ref: RefObject<HTMLDivElement>
  ) => setObligations([...obligations, [file, hash, ref]]);

  const briefHighlight = (file: Filename, hash: ObligationHash) => {
    const idx = _.findIndex(
      obligations,
      ([f, h, _o]) => f === file && h === hash
    );
    const o = obligations[idx][2];

    if (o !== undefined) {
      o.current?.classList.add("highlight");
      setTimeout(() => {
        o.current?.classList.remove("highlight");
      }, 2000);
    }
  };

  // NOTE: this listener should only listen for posted messages, not
  // for things that could be an expected response from a webview request.
  const listener = (e: MessageEvent) => {
    console.log("Received message from extension", e.data);
    const msg = e.data;

    // TODO: none of these messages are actually getting sent yet.
    switch (msg.command) {
      case "bling": {
        briefHighlight(msg.file, msg.hash);
        return;
      }

      case "open-file": {
        setOpenFiles([
          ...openFiles,
          [msg.file, <OpenFile filename={msg.file} />],
        ]);
        return;
      }
      case "close-file": {
        setOpenFiles(
          _.filter(openFiles, ([filename, _]) => filename !== msg.filename)
        );
        return;
      }

      case "invalidate": {
        throw new Error("Invalidation, not yet implemented!");
      }

      // Everthing else must be ignored.
      default:
        return;
    }
  };

  useEffect(() => {
    window.addEventListener("message", listener);
    return () => window.removeEventListener("message", listener);
  }, []);

  const resetState = () => {
    setOpenFiles(
      _.map(openFiles, ([filename, _], i) => {
        return [filename, <OpenFile key={i} filename={filename} />];
      })
    );
  };

  return (
    <ObligationHookContext.Provider value={addRefToList}>
      <Workspace files={openFiles} reset={resetState} />
    </ObligationHookContext.Provider>
  );
};

export default App;
