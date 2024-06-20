import { VSCodeProgressRing } from "@vscode/webview-ui-toolkit/react";
import React, { useEffect, useState } from "react";
import { getHighlighter } from "shiki";

import "./Code.css";

const mkHighlighter = (() => {
  const h = getHighlighter({
    themes: ["dark-plus", "light-plus"],
    langs: ["rust"],
  });
  return async () => await h;
})();

const codeToHtml = async ({ code, lang }: { code: string; lang: string }) => {
  const highlighter = await mkHighlighter();
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
    <span
      className="shiki-wrapper"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
};

export default Code;
