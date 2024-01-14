import { ObligationHash, ObligationsInBody } from "@argus/common/bindings";
import { ExtensionToWebViewMsg, Filename } from "@argus/common/lib";
import _ from "lodash";
import React, { RefObject, useEffect, useState } from "react";



import File, { ObligationHookContext, obligationCardId } from "./File";
import Workspace from "./Workspace";
import { WaitingOn } from "./utilities/WaitingOn";
import { requestFromExtension } from "./utilities/vscode";


const OpenFile = ({ filename }: { filename: Filename }) => {
  const [obligations, setObligations] = useState<
    ObligationsInBody[] | undefined
  >(undefined);

  // FIXME: we only want to load things once, and on invalidation, currently
  // this will run on every render.
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

  const briefHighlight = (file: Filename, hash: ObligationHash) => {
    const elem = document.getElementById(obligationCardId(file, hash));
    const className = "bling";
    if (elem !== null) {
      elem.scrollIntoView();
      elem.classList.add(className);
      setTimeout(() => elem.classList.remove(className), 1000); // Let the emphasis stay for 1 second.
    } else {
      console.error("Couldn't find element to highlight at", file, hash);
    }
  };

  // NOTE: this listener should only listen for posted messages, not
  // for things that could be an expected response from a webview request.
  const listener = (e: MessageEvent) => {
    console.log("Received message from extension", e.data);
    const {
      command,
      payload,
    }: { command: string; payload: ExtensionToWebViewMsg } = e.data;

    if (command != payload.command) {
      console.log("Received message with mismatched commands", e.data);
      return;
    }

    // TODO: none of these messages are actually getting sent yet.
    switch (payload.command) {
      case "bling": {
        briefHighlight(payload.file, payload.oblHash);
        return;
      }

      case "open-file": {
        setOpenFiles((currFiles) => [
          [payload.file, <OpenFile filename={payload.file} />],
          ...currFiles,
        ]);
        return;
      }
      case "close-file": {
        setOpenFiles((currFiles) =>
          _.filter(currFiles, ([filename, _]) => filename !== payload.file)
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
    setOpenFiles(currFiles =>
      _.map(currFiles, ([filename, _], i) => {
        return [filename, <OpenFile key={i} filename={filename} />];
      })
    );
  };

  return <Workspace files={openFiles} reset={resetState} />;
};

export default App;