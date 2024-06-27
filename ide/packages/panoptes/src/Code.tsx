import MonoSpace from "@argus/print/MonoSpace";
import { VSCodeProgressRing } from "@vscode/webview-ui-toolkit/react";
import React, { useEffect, useState } from "react";
import { Highlighter, getHighlighter } from "shiki";

import "./Code.css";

const mkHighlighter = (() => {
  let h: Promise<Highlighter | undefined>;
  try {
    h = getHighlighter({
      themes: ["dark-plus", "light-plus"],
      langs: ["rust"],
    });
  } catch (e: any) {
    console.error("Failed to initialize Shiki highlighter", e);
    h = Promise.resolve(undefined);
  }

  return async () => await h;
})();

const codeToHtml = async ({ code, lang }: { code: string; lang: string }) => {
  const highlighter = await mkHighlighter();
  // TODO: I haven't tested that this works because Shiki has yet to fail :)
  if (!highlighter) {
    return "<pre>" + code + "</pre>";
  }

  return highlighter.codeToHtml(code, {
    lang,
    themes: {
      dark: "dark-plus",
      light: "light-plus",
    },
    defaultColor: "light",
  });
};

const Code = ({ code }: { code: string }) => {
  const [html, setHtml] = useState<string | undefined>();

  useEffect(() => {
    const fetchIt = async () => {
      const html = await codeToHtml({ code, lang: "rust" });
      setHtml(html);
    };

    fetchIt();
  }, [code]);

  return !html ? (
    <VSCodeProgressRing />
  ) : (
    <MonoSpace>
      <span
        className="shiki-wrapper"
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </MonoSpace>
  );
};

export default Code;
