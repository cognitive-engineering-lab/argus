import { BodyBundle } from "@argus/common/bindings";
import { EvaluationMode, Filename, Result } from "@argus/common/lib";
import { ExecNotifyOpts, execNotify as _execNotify } from "@argus/system";
import fs from "fs";
import _ from "lodash";
import path from "path";
import {
  BrowserContext,
  ElementHandle,
  Page,
  chromium,
  selectors,
} from "playwright";
import tmp from "tmp";
//@ts-ignore
import uniqueFilename from "unique-filename";

import { webHtml } from "./page";
import { rootCauses } from "./rootCauses";
import { PORT, fileServer } from "./serve";

declare global {
  var debugging: boolean;
}

// See: https://doc.rust-lang.org/cargo/reference/external-tools.html
//      https://doc.rust-lang.org/rustc/json.html
// for up-to-date information on the structure of diagnostics output.
interface DiagnosticMessage {
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

function isRustcMessage(obj: any): obj is DiagnosticMessage {
  return obj.reason === "compiler-message";
}

function diagnosticFilename(msg: DiagnosticMessage) {
  const mainMsg = _.find(msg.message.spans, span => span.is_primary);
  return mainMsg ? mainMsg.file_name : msg.target.src_path;
}

// TODO: move the "title" from execNotify into the opts, and use the `cmd` if not present.
const execSilent = (cmd: string, args: string[], opts: ExecNotifyOpts) => {
  return _execNotify(cmd, args, opts, (..._args: any[]) => {});
};

async function sleep(waitTime: number) {
  return new Promise(resolve => setTimeout(resolve, waitTime));
}

async function forFileInBundle<T>(
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

async function openPage(
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
    timeout: 30_000,
  });
  return page;
}

async function expandBottomUpView(page: Page) {
  let bs = await page.getByText("Bottom Up").all();
  try {
    // await Promise.all(
    //   _.map(bs, async b => {
    //     try {
    //       await b.waitFor({ state: "visible" });
    //     } catch (e: any) {}
    //   })
    // );
    await Promise.all(
      _.map(bs, async b => {
        try {
          await b.click();
        } catch (e: any) {}
      })
    );
  } catch (e: any) {
    console.debug("Error clicking bottom up", e);
  }
}

// Take bundled argus output and take a screenshot of the tree loaded into the browser.
async function argusScreenshots(
  outDir: fs.PathLike,
  bundles: BodyBundle[],
  title: string = "Argus Output"
) {
  const browser = await chromium.launch({ headless: !global.debugging });
  const context = await browser.newContext();

  const innerDoScreenshot = async (
    out: string,
    filename: Filename,
    bundles: BodyBundle[]
  ) => {
    const page = await openPage(context, filename, bundles, "rank");
    await expandBottomUpView(page);
    await page.screenshot({ path: out, fullPage: true });
  };

  return forFileInBundle(bundles, async (filename, bundles) => {
    const outfile = uniqueFilename(outDir) + ".png";
    await innerDoScreenshot(outfile, filename, bundles);
    return { filename, outfile };
  });
}

async function argusData(dir: string) {
  const argusOutput = await execSilent("cargo", ["argus", "bundle"], {
    cwd: dir,
  });
  const bundles: Result<BodyBundle[]> = JSON.parse(argusOutput);
  return bundles;
}

async function cargoMessages(dir: string) {
  const cargoOutput = await execSilent(
    "cargo",
    ["check", "--message-format", "json"],
    { cwd: dir, ignoreExitCode: true }
  );
  const cargoOutputLns = cargoOutput.split("\n");
  const output = _.map(cargoOutputLns, JSON.parse);
  const rustcMessages = _.filter(output, isRustcMessage);
  return _.groupBy(rustcMessages, msg => diagnosticFilename(msg));
}

function ensureDir(dir: fs.PathLike) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

// When chumsky is working just read the directory contents
const testCases = [
  "axum",
  "bevy",
  "diesel",
  "easy_ml",
  "entrait",
  "nalgebra",
  "uom",
] as const;

async function runForJson<T>(func: () => Promise<T>) {
  return JSON.stringify(JSON.stringify(await func()));
}

async function runRandom(N: number) {
  fileServer().listen(PORT);
  const workspaceDir = path.resolve(__dirname, "..", "workspaces");
  const browser = await chromium.launch({ headless: !global.debugging });
  const context = await browser.newContext();
  context.setDefaultTimeout(5_000);

  const innerF = async (workspace: string, causes: any[]) => {
    const fullSubdir = path.resolve(workspaceDir, workspace);
    const argusBundles = await argusData(fullSubdir);

    if (!("Ok" in argusBundles)) throw new Error("Argus failed");
    const fileBundles = (await forFileInBundle(
      argusBundles.Ok,
      async (filename, bundles) => [filename, bundles]
    )) as [string, BodyBundle[]][];

    let results = [];
    for (const [filename, bundles] of fileBundles) {
      const cause = _.find(causes, cause => filename.endsWith(cause.file));

      if (!cause) {
        console.error(`No cause found for ${filename} in ${workspace}`);
        continue;
      }
      console.debug(`Running ${N} samples for ${filename}`);
      let ranks = await Promise.all(
        _.times(N, async () => {
          const page = await openPage(context, filename, bundles, "random");
          await sleep(3000);
          await expandBottomUpView(page);
          await sleep(3000);
          try {
            const goals = await page
              .locator(".BottomUpArea .EvalGoal")
              .filter({ hasText: cause.message })
              .all();

            const ranksStr = await Promise.all(
              _.map(goals, goal => goal.getAttribute("data-rank"))
            );

            return _.min(_.map(_.compact(ranksStr), r => Number(r)));
          } catch (e) {
            return undefined;
          } finally {
            await page.close();
          }
        })
      );

      const noUndef = _.compact(ranks);
      if (noUndef.length === 0) {
        console.error(`No ranks found for ${filename} in ${workspace}`);
      }

      results.push({
        workspace,
        filename,
        cause: cause.message,
        ranks: noUndef,
      });
    }

    return results;
  };

  let results = [];
  for (const { workspace, causes } of rootCauses) {
    results.push(await innerF(workspace, causes));
  }

  return _.flatten(results);
}

async function runEvaluation() {
  fileServer().listen(PORT);
  const workspaceDir = path.resolve(__dirname, "..", "workspaces");
  const browser = await chromium.launch({ headless: !global.debugging });
  const context = await browser.newContext();
  const results = await Promise.all(
    _.map(rootCauses, async ({ workspace, causes }) => {
      const fullSubdir = path.resolve(workspaceDir, workspace);
      const argusBundles = await argusData(fullSubdir);

      if (!("Ok" in argusBundles)) throw new Error("Argus failed");
      return forFileInBundle(argusBundles.Ok, async (filename, bundles) => {
        const cause = _.find(causes, cause => filename.endsWith(cause.file)) as
          | { file: string; message: string }
          | undefined;

        if (!cause) return;
        const page = await openPage(context, filename, bundles, "rank");

        await sleep(5000);
        await expandBottomUpView(page);
        await sleep(5000);

        const goals = await page
          .locator(".BottomUpArea .EvalGoal")
          .filter({ hasText: cause.message })
          .all();

        console.debug(
          `Found ${filename}:${goals.length} goals ${cause.message}`
        );

        const ranksStr = await Promise.all(
          _.map(goals, goal => goal.getAttribute("data-rank"))
        );
        const rank = _.min(_.map(_.compact(ranksStr), r => Number(r)));

        const numberTreeNodes = _.max(
          _.flatten(
            _.map(bundles, bundle =>
              _.map(_.values(bundle.trees), tree => tree.nodes.length)
            )
          )
        );

        await page.close();
        return {
          workspace,
          filename,
          cause: cause.message,
          numberTreeNodes,
          rank,
        };
      });
    })
  );

  return _.filter(_.compact(_.flatten(results)), v => v.rank !== undefined);
}

async function outputInDir(resultsDir: string) {
  fileServer().listen(PORT);
  ensureDir(resultsDir);

  const workspaceDir = path.resolve(__dirname, "..", "workspaces");

  const doForSubdir = async (outdir: string, subdir: string) => {
    const fullSubdir = path.resolve(workspaceDir, subdir);
    const outDir = path.resolve(outdir, subdir);
    ensureDir(outDir);

    const [rustcMessages, argusBundles] = await Promise.all([
      cargoMessages(fullSubdir),
      argusData(fullSubdir),
    ]);

    const result =
      "Ok" in argusBundles
        ? await (async () => {
            const screenshots = await argusScreenshots(
              outDir,
              argusBundles.Ok,
              subdir
            );
            return _.map(screenshots, ({ filename, outfile }) => {
              const diagnostics =
                _.find(rustcMessages, (_msgs, src) => {
                  return src.endsWith(filename) || filename.endsWith(src);
                }) ?? [];
              return { filename, argusScreenshotFn: outfile, diagnostics };
            });
          })()
        : argusBundles;

    return { test: subdir, result };
  };

  return await Promise.all(
    _.map(testCases, subdir => doForSubdir(resultsDir, subdir))
  );
}

async function runScreenshots(cacheDirectory: string) {
  const results = await outputInDir(cacheDirectory);
  const mapFile = path.resolve(cacheDirectory, "results-map.json");
  fs.writeFileSync(mapFile, JSON.stringify(results));
}

async function main() {
  global.debugging = _.includes(process.argv, "--debug");
  const argv = _.filter(process.argv, arg => arg !== "--debug");

  switch (argv[2]) {
    case "-r": {
      const N = Number(argv[3] ?? "10");
      await runForJson(() => runRandom(N)).then(console.log);
      break;
    }
    case "-h": {
      await runForJson(() => runEvaluation()).then(console.log);
      break;
    }
    case "-s": {
      const cacheDirectory = argv[3];
      if (cacheDirectory === undefined)
        throw new Error("Must provide a cache directory");
      await runScreenshots(cacheDirectory);
    }
    default:
      throw new Error("Invalid CLI argument");
  }
  process.exit(0);
}

main().catch(err => {
  console.error("Error running evaluation");
  console.error(err);
  process.exit(1);
});
