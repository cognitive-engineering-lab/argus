import MonoSpace from "@argus/print/MonoSpace";
import { VSCodeProgressRing } from "@vscode/webview-ui-toolkit/react";
import React, { useEffect, useState } from "react";

import { type HighlighterCore, createHighlighterCore } from "@shikijs/core";
import { createOnigurumaEngine } from "@shikijs/engine-oniguruma";

import rust from "@shikijs/langs/rust";
import dplus from "@shikijs/themes/dark-plus";
import ldark from "@shikijs/themes/github-light-default";
import lplus from "@shikijs/themes/light-plus";
import cdark from "@shikijs/themes/synthwave-84";
import wasm from "shiki/wasm";

import "./Code.css";

const ARGUS_THEMES = {
  dark: "dark-plus",
  light: "light-plus",
  "contrast-dark": "synthwave-84",
  "contrast-light": "github-light-default"
};

const mkHighlighter = (() => {
  let h: Promise<HighlighterCore | undefined>;
  try {
    h = createHighlighterCore({
      themes: [dplus, lplus, cdark, ldark],
      langs: [rust],
      engine: createOnigurumaEngine(wasm)
    });
  } catch (e: any) {
    console.error("Failed to initialize Shiki highlighter", e);
    h = Promise.resolve(undefined);
  }

  return async () => await h;
})();

const codeToHtml = async ({ code, lang }: { code: string; lang: string }) => {
  let highlighter: HighlighterCore | undefined;

  try {
    highlighter = await mkHighlighter();
    if (!highlighter) throw new Error("Highlighter not initialized");
  } catch (e: any) {
    return `<pre>${code}</pre>`;
  }

  return highlighter.codeToHtml(code, {
    lang,
    themes: ARGUS_THEMES,
    defaultColor: "light"
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
        /* biome-ignore lint/security/noDangerouslySetInnerHtml: shiki */
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </MonoSpace>
  );
};

export default Code;
