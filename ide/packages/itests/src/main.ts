import Mocha from "mocha";

import "./add.test.ts";

export async function run(): Promise<void> {
  console.debug("RUNNING TESTS");

  // Create the mocha test
  const mocha = new Mocha({
    ui: "tdd",
    color: true,
  });

  return new Promise(function (resolve, reject) {
    try {
      // FIXME: note to self.
      //
      // The global Mocha context isn't getting set, because the EVENT_FILE_PRE_REQUIRE
      // isn't getting emitted for "loaded" files. This normally gets emitted after
      // files are added to Mocha, and then they are prepared before running.
      // Manually emitting the event doesn't do anything, because the problem is having the
      // tests bundled and loaded in the first place.
      //
      // ste.emit(Suite.constants.EVENT_FILE_PRE_REQUIRE, mocha);
      //
      // Using vitest isn't working either for similar reasons. It has an added problem
      // of introducing @types/chai mismatch problems due to namespace exports. This is
      // an open issue on the vitest repo.

      // Run the mocha test
      mocha.timeout(10000);

      mocha.run(function (failures) {
        if (failures > 0) {
          reject(new Error(`${failures} tests failed.`));
        } else {
          resolve();
        }
      });
    } catch (err) {
      reject(err);
    }
  });
}
