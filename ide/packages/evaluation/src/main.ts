import _ from "lodash";

import fs from "node:fs";

import { run as runBasic } from "./basic";
import { run as runRandom } from "./random";
import { rootCauses as TEST_CAUSES } from "./rootCauses";
import { PORT, withServerOnPort } from "./serve";
import { run as runVisual } from "./visual";

declare global {
  var debugging: boolean;
  var testMatcher: string;
  var outputFile: string;
}

async function runForJson<T>(func: () => Promise<T>) {
  return JSON.stringify(JSON.stringify(await func()));
}

async function main() {
  const hasTestMatcher = _.find(process.argv, arg => arg.startsWith("--test="));
  const hasOutputFile = _.find(process.argv, arg => arg.startsWith("--ofile="));

  global.debugging = _.includes(process.argv, "--debug");
  global.outputFile = hasOutputFile
    ? hasOutputFile?.split("=")[1]
    : "heuristic-precision.csv";
  global.testMatcher = hasTestMatcher
    ? hasTestMatcher.split("=")[1]
    : "[\\s\\S]*";

  const argv = _.filter(
    process.argv,
    arg => arg !== "--debug" && !arg.startsWith("--test")
  );

  switch (argv[2]) {
    case "-r": {
      const N = Number(argv[3] ?? "10");
      await withServerOnPort(PORT, () =>
        runForJson(() => runRandom(N, TEST_CAUSES))
      ).then(console.log);
      break;
    }
    case "-h": {
      await withServerOnPort(PORT, () => runBasic(TEST_CAUSES)).then(
        writeToCSV(global.outputFile)
      );
      break;
    }
    case "-s": {
      const cacheDirectory = argv[3];
      if (cacheDirectory === undefined)
        throw new Error("Must provide a cache directory");
      await withServerOnPort(PORT, () => runVisual(cacheDirectory));
      break;
    }
    default:
      throw new Error("Invalid CLI argument");
  }
  process.exit(0);
}

const writeToCSV = (filename: string) => (os: Object[]) => {
  if (os.length === 0) return;

  const names = Object.keys(os[0]).join(",");
  const data = _.map(os, obj => _.map(_.values(obj), JSON.stringify).join(","));
  const all = [names, ...data];
  return fs.writeFileSync(filename, all.join("\n"));
};

main().catch(err => {
  console.error("Error running evaluation");
  console.error(err);
  process.exit(1);
});
