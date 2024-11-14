import _ from "lodash";

import fs from "node:fs";
import { run as runBasic } from "./basic";
import { rootCauses as TEST_CAUSES } from "./rootCauses";
import { PORT, withServerOnPort } from "./serve";
import { run as runVisual } from "./visual";

declare global {
  var debugging: boolean;
  var testMatcher: string;
  var outputFile: string;
  var rankBy: string;
}

async function runForJson<T>(func: () => Promise<T>) {
  return JSON.stringify(JSON.stringify(await func()));
}

async function main() {
  const hasTestMatcher = _.find(process.argv, arg => arg.startsWith("--test="));
  const hasOutputFile = _.find(process.argv, arg => arg.startsWith("--ofile="));
  let hasRankBy = _.find(process.argv, arg => arg.startsWith("--rankBy="));

  global.debugging = _.includes(process.argv, "--debug");
  global.testMatcher = hasTestMatcher
    ? hasTestMatcher.split("=")[1]
    : "[\\s\\S]*";
  global.rankBy = hasRankBy ? hasRankBy.split("=")[1] : "inertia";
  global.outputFile = hasOutputFile
    ? hasOutputFile?.split("=")[1]
    : `heuristic-precision[${global.rankBy}].csv`;

  const argv = _.filter(
    process.argv,
    arg => arg !== "--debug" && !arg.startsWith("--test")
  );

  switch (argv[2]) {
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
