import path from "node:path";
import _ from "lodash";
import { chromium } from "playwright";

import type { RootCause } from "./rootCauses";
import {
  argusData,
  expandBottomUpView,
  forFileInBundle,
  openPage,
  sleep,
  testCases
} from "./utils";

async function createWorkspaceRunner() {
  // Shared state among all runs
  const workspaceDir = path.resolve(__dirname, "..", "workspaces");
  const browser = await chromium.launch({ headless: !global.debugging });
  const context = await browser.newContext();

  return async ({ workspace, causes }: RootCause) => {
    const fullSubdir = path.resolve(workspaceDir, workspace);
    const argusBundles = await argusData(fullSubdir);

    if (!("Ok" in argusBundles)) throw new Error("Argus failed");
    return forFileInBundle(argusBundles.Ok, async (filename, bundles) => {
      const cause = _.find(causes, cause => filename.endsWith(cause.file)) as
        | { file: string; message: string }
        | undefined;

      if (!cause) {
        console.debug(`MISSING: cause ${workspace}/${filename}`);
        return;
      }
      const page = await openPage(context, filename, bundles);

      await sleep(5000);
      await expandBottomUpView(page);
      await sleep(5000);

      const goals = await page
        .locator(".BottomUpArea .EvalGoal")
        .filter({ hasText: cause.message })
        .all();

      console.debug(`Found ${filename}:${goals.length} goals ${cause.message}`);

      const ranksStr = await Promise.all(
        _.map(goals, goal => goal.getAttribute("data-rank"))
      );
      const rank = _.min(_.map(_.compact(ranksStr), r => Number(r))) ?? -1;

      const numberTreeNodes =
        _.max(
          _.flatten(
            _.map(bundles, bundle =>
              _.map(_.values(bundle.trees), tree => tree.nodes.length)
            )
          )
        ) ?? -1;

      await page.close();
      return {
        workspace,
        filename,
        rank
      };
    });
  };
}

export async function run(rootCauses: RootCause[]) {
  const runForWorkspace = await createWorkspaceRunner();

  const tcs = testCases();
  const filteredCauses = _.filter(rootCauses, ({ workspace }) =>
    _.includes(tcs, workspace)
  );

  const results = await Promise.all(_.map(filteredCauses, runForWorkspace));
  const flatResults = _.flatten(results);
  return _.filter(_.compact(flatResults), v => v.rank !== undefined);
}
