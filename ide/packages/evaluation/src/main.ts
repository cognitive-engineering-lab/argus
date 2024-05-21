import _ from "lodash";

import { run as runBasic } from "./basic";
import { run as runRandom } from "./random";
import { rootCauses as TEST_CAUSES } from "./rootCauses";
import { PORT, withServerOnPort } from "./serve";
import { run as runVisual } from "./visual";

declare global {
  var debugging: boolean;
  var testMatcher: string;
}

async function runForJson<T>(func: () => Promise<T>) {
  return JSON.stringify(JSON.stringify(await func()));
}

async function main() {
  global.debugging = _.includes(process.argv, "--debug");
  const hasTestMatcher = _.find(process.argv, arg => arg.startsWith("--test="));
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
      await withServerOnPort(PORT, () =>
        runForJson(() => runBasic(TEST_CAUSES))
      ).then(console.log);
      break;
    }
    case "-s": {
      const cacheDirectory = argv[3];
      if (cacheDirectory === undefined)
        throw new Error("Must provide a cache directory");
      await withServerOnPort(PORT, () => runVisual(cacheDirectory));
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
