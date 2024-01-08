import {
  ExtensionReturn,
  ExtensionToWebViewMsg,
  WebViewToExtensionMsg,
} from "@argus/common";
import { messageHandler } from "@estruyf/vscode/dist/client";

// TODO: how can we force T === body.command?
export const requestFromExtension = <
  T extends ExtensionToWebViewMsg["command"]
>(
  body: WebViewToExtensionMsg
): Promise<ExtensionReturn<T>> => {
  return messageHandler.request<ExtensionReturn<T>>(body.command, body);
};

export const postToExtension = (body: WebViewToExtensionMsg) => {
  return messageHandler.send(body.command, body);
};
