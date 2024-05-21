import { BodyBundle } from "@argus/common/bindings";
import { Filename } from "@argus/common/lib";
import { execNotify as _execNotify } from "@argus/system";
import fs from "fs";
import _ from "lodash";
import path from "path";
import { chromium } from "playwright";
//@ts-ignore
import uniqueFilename from "unique-filename";

import {
  argusData,
  diagnosticFilename,
  execSilent,
  expandBottomUpView,
  forFileInBundle,
  isRustcMessage,
  openPage,
  testCases,
} from "./utils";

function ensureDir(dir: fs.PathLike) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
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

export async function run(cacheDirectory: string) {
  const results = await outputInDir(cacheDirectory);
  const mapFile = path.resolve(cacheDirectory, "results-map.json");
  fs.writeFileSync(mapFile, JSON.stringify(results));
}

async function outputInDir(resultsDir: string) {
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
    _.map(testCases(), subdir => doForSubdir(resultsDir, subdir))
  );
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
