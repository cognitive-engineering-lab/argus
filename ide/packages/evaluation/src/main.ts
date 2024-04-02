import { BodyBundle } from "@argus/common/bindings";
import { Filename, Result } from "@argus/common/lib";
import { ExecNotifyOpts, execNotify as _execNotify } from "@argus/system";
import fs from "fs";
import _ from "lodash";
import path from "path";
import { chromium } from "playwright";
import tmp from "tmp";
//@ts-ignore
import uniqueFilename from "unique-filename";

import { webHtml } from "./page";
import { PORT, fileServer } from "./serve";

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

// Take bundled argus output and take a screenshot of the tree loaded into the browser.
async function argusScreenshots(
  outDir: fs.PathLike,
  bundles: BodyBundle[],
  title: string = "Argus Output"
) {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext();

  const innerDoScreenshot = async (
    tmpobj: tmp.FileResult,
    out: string,
    filename: Filename,
    bundles: BodyBundle[]
  ) => {
    const html = webHtml(title, filename, bundles);
    fs.writeSync(tmpobj.fd, html);
    const page = await context.newPage();
    await page.goto(`file://${tmpobj.name}`);
    await sleep(6000);
    await page.screenshot({ path: out, fullPage: false });
  };

  const groupedByFilename = _.groupBy(bundles, b => b.filename);
  return await Promise.all(
    _.map(groupedByFilename, async (bundles, filename) => {
      const tmpobj = tmp.fileSync({ postfix: ".html" });
      const outfile = uniqueFilename(outDir) + ".png";
      await innerDoScreenshot(tmpobj, outfile, filename, bundles);
      return { filename, outfile };
    })
  );
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

async function main() {
  const cacheDirectory = process.argv[2];
  if (cacheDirectory === undefined) {
    throw new Error("Must provide a cache directory");
  }

  const results = await outputInDir(cacheDirectory);
  const mapFile = path.resolve(cacheDirectory, "results-map.json");
  fs.writeFileSync(mapFile, JSON.stringify(results));
  process.exit(0);
}

main().catch(err => {
  console.error("Error running evaluation");
  console.error(err);
  process.exit(1);
});
