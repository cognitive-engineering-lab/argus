import path from "node:path";
import type { BodyBundle } from "@argus/common/bindings";
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

async function createInnerRun(N: number) {
  const workspaceDir = path.resolve(__dirname, "..", "workspaces");
  const browser = await chromium.launch({ headless: !global.debugging });
  const context = await browser.newContext();
  context.setDefaultTimeout(5_000);

  return async (workspace: string, causes: any[]) => {
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
        ranks: noUndef
      });
    }

    return results;
  };
}

export async function run(N: number, rootCauses: RootCause[]) {
  const innerF = await createInnerRun(N);
  const tcs = testCases();

  let results = [];
  for (const { workspace, causes } of rootCauses) {
    if (!_.includes(tcs, workspace)) continue;
    results.push(await innerF(workspace, causes));
  }

  return _.flatten(results);
}
