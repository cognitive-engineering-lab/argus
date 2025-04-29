import _ from "lodash";

import type { BodyBundle, ProofNodeIdx } from "./bindings";
import { rangeContains } from "./func";
import {
  type PanoptesToSystemCmds,
  type PanoptesToSystemMsg,
  type SystemReturn,
  isPanoMsgTree
} from "./lib";

export type InfoWrapperProps = {
  n: ProofNodeIdx;
  reportActive: (b: boolean) => void;
};

export type InfoWrapper = React.FC<InfoWrapperProps>;

export interface TreeRenderParams {
  Wrappers?: InfoWrapper[];
  styleEdges?: boolean;
  startOpenP?: (n: ProofNodeIdx) => boolean;
  onMount?: () => void;
}

export interface MessageSystem {
  postData<T extends PanoptesToSystemCmds>(
    command: T,
    body: Omit<PanoptesToSystemMsg<T>, "command">
  ): void;

  requestData<T extends PanoptesToSystemCmds>(
    command: T,
    body: Omit<PanoptesToSystemMsg<T>, "command">
  ): Promise<SystemReturn<T>>;
}

export interface VSCodeMessageHandler {
  send(message: string, payload?: any): void;
  request<T>(message: string, payload?: any): Promise<T>;
}

export function createClosedMessageSystem(bodies: BodyBundle[]): MessageSystem {
  const systemMap = _.groupBy(bodies, bundle => bundle.filename);
  return {
    postData<T extends PanoptesToSystemCmds>(
      _command: T,
      _body: Omit<PanoptesToSystemMsg<T>, "command">
    ) {
      // Intentionally blank, no system to post to.
    },

    requestData<T extends PanoptesToSystemCmds>(
      command: T,
      bodyOmit: Omit<PanoptesToSystemMsg<T>, "command">
    ) {
      return new Promise<SystemReturn<T>>((resolve, reject) => {
        const body = { command, ...bodyOmit };

        if (!isPanoMsgTree(body)) {
          return reject(new Error(`"Invalid message type" ${command}`));
        }

        const rangesInFile = systemMap[body.file];
        if (rangesInFile === undefined) {
          return reject(
            new Error(`file messages not found for '${body.file}'`)
          );
        }

        const obligationRange = body.range;
        const foundBodies = _.filter(rangesInFile, bundle =>
          rangeContains(bundle.body.range, obligationRange)
        );
        if (foundBodies.length === 0) {
          return reject(new Error(`body in range ${body.range} not found`));
        }

        const tree = _.head(
          _.compact(
            _.map(foundBodies, found => found.trees[body.predicate.hash])
          )
        );
        if (tree === undefined) {
          console.error(
            "Tree not found in bodies",
            foundBodies,
            "hash",
            body.predicate.hash
          );
          return reject(new Error("Obligation hash not found in maps"));
        }

        const treeReturn = { tree } as SystemReturn<"tree">;
        resolve(treeReturn as SystemReturn<T>);
      });
    }
  };
}
