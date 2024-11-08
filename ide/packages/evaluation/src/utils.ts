import fs from "node:fs";
import type { BodyBundle } from "@argus/common/bindings";
import type { EvaluationMode, Filename, Result } from "@argus/common/lib";
import { type ExecNotifyOpts, execNotify as _execNotify } from "@argus/system";
import _ from "lodash";
import type { BrowserContext, Page } from "playwright";
import tmp from "tmp";

import { webHtml } from "./page";

export async function argusData(dir: string) {
  const argusOutput = await execSilent("cargo", ["argus", "bundle"], {
    cwd: dir
  });
  const bundles: Result<BodyBundle[]> = JSON.parse(argusOutput);
  return bundles;
}

// TODO: move the "title" from execNotify into the opts, and use the `cmd` if not present.
export const execSilent = async (
  cmd: string,
  args: string[],
  opts: ExecNotifyOpts
) => {
  return await _execNotify(cmd, args, opts, (..._args: any[]) => {});
};

export async function sleep(waitTime: number) {
  return new Promise(resolve => setTimeout(resolve, waitTime));
}

export function testCases() {
  // When chumsky is working just read the directory contents
  const _testCases = [
    "axum",
    "bevy",
    "diesel",
    // "easy_ml",
    // "chumsky",
    "entrait",
    "nalgebra",
    "uom"
  ] as const;

  return _testCases.filter(test => test.match(global.testMatcher));
}

// See: https://doc.rust-lang.org/cargo/reference/external-tools.html
//      https://doc.rust-lang.org/rustc/json.html
// for up-to-date information on the structure of diagnostics output.
export interface DiagnosticMessage {
  reason: "compiler-message";
  target: {
    src_path: string;
  };
  message: {
    rendered: string;
    message: string;
    spans: { file_name: string; is_primary: boolean }[];
  };
}

export function diagnosticFilename(msg: DiagnosticMessage) {
  const mainMsg = _.find(msg.message.spans, span => span.is_primary);
  return mainMsg ? mainMsg.file_name : msg.target.src_path;
}

export function isRustcMessage(obj: any): obj is DiagnosticMessage {
  return obj.reason === "compiler-message";
}

export async function expandBottomUpView(page: Page) {
  let bs = await page.getByText("Bottom Up").all();
  try {
    await Promise.all(_.map(bs, b => b.click()));
  } catch (e: any) {
    console.debug("Error clicking bottom up", e);
  }
}

export async function forFileInBundle<T>(
  bundles: BodyBundle[],
  f: (filename: Filename, bundles: BodyBundle[]) => Promise<T>
) {
  const groupedByFilename = _.groupBy(bundles, b => b.filename);
  return await Promise.all(
    _.map(groupedByFilename, async (bundles, filename) => {
      return f(filename, bundles);
    })
  );
}

export async function openPage(
  context: BrowserContext,
  filename: string,
  bundles: BodyBundle[],
  evalMode: EvaluationMode
) {
  const tmpobj = tmp.fileSync({ postfix: ".html" });
  const html = webHtml("EVAL", filename, bundles, evalMode);
  fs.writeSync(tmpobj.fd, html);
  const page = await context.newPage();
  await page.goto(`file://${tmpobj.name}`, {
    waitUntil: "domcontentloaded",
    timeout: 30_000
  });
  return page;
}
