import {
  BodyHash,
  CharRange,
  ObligationHash,
  SerializedTree,
} from "@argus/common/bindings";
import {
  Filename,
  PanoptesToSystemCmds,
  PanoptesToSystemMsg,
  SystemReturn,
  SystemToPanoptesCmds,
  SystemToPanoptesMsg,
  isPanoMsgTree,
} from "@argus/common/lib";
import { messageHandler } from "@estruyf/vscode/dist/client";
import _ from "lodash";
import { createContext } from "react";

export const MessageSystemContext = createContext<MessageSystem | undefined>(
  undefined
);

export interface MessageSystem {
  postData<T extends PanoptesToSystemCmds>(body: PanoptesToSystemMsg<T>): void;

  // TODO: how can we force T === body.command?
  requestData<T extends PanoptesToSystemCmds>(
    body: PanoptesToSystemMsg<T>
  ): Promise<SystemReturn<T>>;
}

export const vscodeMessageSystem: MessageSystem = {
  postData<T extends PanoptesToSystemCmds>(body: PanoptesToSystemMsg<T>) {
    return messageHandler.send(body.command, body);
  },

  requestData<T extends PanoptesToSystemCmds>(body: PanoptesToSystemMsg<T>) {
    return messageHandler.request<SystemReturn<T>>(body.command, body);
  },
};

export type SystemPartialMap = Map<
  Filename,
  [CharRange, Map<ObligationHash, SerializedTree>]
>;

export function createClosedMessageSystem(
  systemMap: SystemPartialMap
): MessageSystem {
  return {
    postData<T extends PanoptesToSystemCmds>(_body: PanoptesToSystemMsg<T>) {
      // Intentionally blank, no system to post to.
    },

    requestData<T extends PanoptesToSystemCmds>(body: PanoptesToSystemMsg<T>) {
      return new Promise<SystemReturn<T>>((resolve, reject) => {
        if (!isPanoMsgTree(body)) {
          return reject();
        }

        const rangesInFile = systemMap.get(body.file) as
          | [CharRange, Map<ObligationHash, SerializedTree>][]
          | undefined;
        if (rangesInFile === undefined) {
          return reject();
        }

        const targetRange = body.range;
        const found = _.find(rangesInFile, ([range, _oblMap]) => {
          const linesOutOfBounds =
            range.end.line < targetRange.start.line ||
            range.start.line > targetRange.end.line;
          const linesEq =
            range.start.line === targetRange.start.line &&
            range.end.line === targetRange.end.line;
          const colsOutOfBounds =
            range.end.column < targetRange.start.column ||
            range.start.column > targetRange.end.column;
          return !(linesOutOfBounds || (linesEq && colsOutOfBounds));
        });
        if (found === undefined) {
          return reject();
        }

        const [_range, oblMap] = found;
        const tree = oblMap.get(body.predicate.hash);
        if (tree === undefined) {
          return reject();
        }

        const treeReturn = { tree } as SystemReturn<"tree">;
        resolve(treeReturn as SystemReturn<T>);
      });
    },
  };
}
