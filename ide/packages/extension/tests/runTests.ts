import { downloadAndUnzipVSCode, runTests } from "@vscode/test-electron";
import fs from "fs";
import path from "path";
import { test } from "vitest";

import { TEST_WORKSPACES } from "./constants";

async function runOnWorkspace(workspace: string) {
  // The folder containing the Extension Manifest package.json
  // Passed to `--extensionDevelopmentPath`
  const extensionDevelopmentPath = path.resolve(__dirname, "../");

  // The path to the extension test runner script
  // Passed to --extensionTestsPath
  const extensionTestsPath = path.resolve(__dirname, "./suite/index");

  // Get all .rs files in ${workspace}/src/**.rs
  const workspaceFiles = path.resolve(workspace, "src/**/*.rs");

  const launchArgs = ["--disable-extensions", workspace, ...workspaceFiles];

  // Download VS Code, unzip it and run the integration test
  await runTests({
    version: "stable",
    launchArgs,
    extensionDevelopmentPath,
    extensionTestsPath,
  });
}

async function main() {
  const workspaceDirectory = path.resolve(__dirname, TEST_WORKSPACES);

  // Get all subdirectories of TEST_WORKSPACES
  const workspaces = fs.readdirSync(workspaceDirectory);

  // for each subdirectory of TEST_WORKSPACES, run the tests
  const testingWorkspaces = workspaces.map(async workspace => {
    // FIXME: remove after testing
    if (workspace === "bevy") {
      await runOnWorkspace(path.resolve(workspaceDirectory, workspace)).catch(
        err => {
          console.error(`Failed to run tests on workspace: ${workspace}`);
          console.error(err);
          throw new Error(err);
        }
      );
    }
  });

  await Promise.all(testingWorkspaces);
}

test("Run extension tests", async () => {
  await main();
}, 1000000);
