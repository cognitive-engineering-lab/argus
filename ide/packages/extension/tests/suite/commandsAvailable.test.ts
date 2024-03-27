import { expect } from "chai";
import { suite, test } from "mocha";
import vscode from "vscode";
import waitForExpect from "wait-for-expect";

const argusCommandsExist = async () => {
  const commands = await vscode.commands.getCommands();
  return commands.find(command => command.startsWith("argus."));
};

suite("Argus installation tests", () => {
  const timeout = 50 * 1000;

  test("installs Argus", async () => {
    const interval = 1 * 1000;

    // Wait for Argus commands to exist, polling every second for 50 seconds
    await waitForExpect(
      async () => {
        expect(await argusCommandsExist()).is.not.undefined;
      },
      timeout,
      interval
    );
  }).timeout(timeout);
});
