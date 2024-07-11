import path from "node:path";
import type {
  ArgusArgs,
  ArgusCliOptions,
  ArgusResult,
  ArgusReturn,
  CallArgus,
  Result
} from "@argus/common/lib";
import {
  type ExecNotifyOpts,
  type RustcToolchain,
  execNotify as _execNotify,
  cargoCommand,
  getCargoOpts
} from "@argus/system";
import type { CancelablePromise as CPromise } from "cancelable-promise";
import _ from "lodash";
import open from "open";
import vscode from "vscode";

import type { CtxInit } from "./ctx";
import { ARGUS_ERR_LOG_KEY } from "./errors";
import { log } from "./logging";
import { isStatusBarState } from "./statusbar";

declare const VERSION: string;
declare const TOOLCHAIN: {
  channel: string;
  components: string[];
};

let CTX: CtxInit | undefined;

// TODO: all of the calls to `showInformationMessage` should be replaced
// with some automatic message on the statusBar indicator.

function getCurrentToolchain(): RustcToolchain {
  return {
    version: VERSION,
    ...TOOLCHAIN
  };
}

const findWorkspaceRoot = async (): Promise<string | null> => {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) {
    log("No folders exist");
    return null;
  }

  const hasCargoToml = async (dir: string) => {
    const manifestPath = path.join(dir, "Cargo.toml");
    const manifestUri = vscode.Uri.file(manifestPath);
    try {
      await vscode.workspace.fs.stat(manifestUri);
      return true;
    } catch (e) {
      return false;
    }
  };

  const folderPath = folders[0].uri.fsPath;
  if (await hasCargoToml(folderPath)) return folderPath;

  const activeEditor = vscode.window.activeTextEditor;
  if (!activeEditor) {
    log("No active editor exists");
    return null;
  }

  const activeFilePath = activeEditor.document.fileName;
  log(`Looking for workspace root between ${folderPath} and ${activeFilePath}`);

  const components = path.relative(folderPath, activeFilePath).split(path.sep);
  const folderSubdirTil = (idx: number) =>
    path.join(folderPath, ...components.slice(0, idx));
  const prefixHasToml = await Promise.all(
    _.range(components.length).map(idx => ({
      idx,
      has: hasCargoToml(folderSubdirTil(idx))
    }))
  );
  const entry = prefixHasToml.find(({ has }) => has);
  if (entry === undefined) return null;

  return folderSubdirTil(entry.idx);
};

const execNotify = (
  cmd: string,
  args: string[],
  title: string,
  opts?: ExecNotifyOpts
) => {
  return _execNotify(
    cmd,
    args,
    {
      title,
      ...opts
    },
    (...args) => log("Argus output", ...args),
    state =>
      isStatusBarState(state) ? CTX?.statusBar.setState(state, title) : {}
  );
};

const checkVersionAndInstall = async (
  config: RustcToolchain,
  workspaceRoot: string,
  cargo: string,
  cargoArgs: string[]
): Promise<boolean> => {
  let version: string;
  try {
    version = await execNotify(
      cargo,
      [...cargoArgs, "argus", "-V"],
      "Waiting for Argus...",
      {
        cwd: workspaceRoot
      }
    );
  } catch (e) {
    version = "";
  }

  if (version !== config.version) {
    log(
      `Argus binary version ${version} does not match expected IDE version ${VERSION}`
    );
    const components = config.components.flatMap(c => ["-c", c]);
    try {
      vscode.window.showInformationMessage(
        "Installing nightly Rust (this may take a minute)"
      );
      await execNotify(
        "rustup",
        [
          "toolchain",
          "install",
          config.channel,
          "--profile",
          "minimal",
          ...components
        ],
        "Installing nightly Rust..."
      );
    } catch (e: any) {
      const choice = await vscode.window.showErrorMessage(
        'Argus failed to install because rustup failed. Click "Show fix" to resolve, or click "Dismiss" to attempt installation later.',
        "Show fix",
        "Dismiss"
      );

      if (choice === "Show fix") {
        open(
          "https://github.com/cognitive-engineering-lab/argus/blob/master/README.md#rustup-fails-on-install"
        );
        await vscode.window.showInformationMessage(
          'Click "Continue" once you have completed the fix.',
          "Continue"
        );
      } else {
        return false;
      }
    }

    vscode.window.showInformationMessage(
      "Installing Argus from source (this may take a minute)"
    );
    await execNotify(
      cargo,
      [
        ...cargoArgs,
        "install",
        "argus-cli",
        "--version",
        config.version,
        "--force"
      ],
      "Installing Argus from source... (this may take a minute)"
    );

    if (version === "") {
      vscode.window.showInformationMessage("Argus has successfully installed!");
    }
  }

  return true;
};

export async function setup(context: CtxInit): Promise<CallArgus | null> {
  // FIXME: this is a hack to give the `context` dynamic scope in this file.
  CTX = context;

  log("Getting workspace root");

  const workspaceRoot = await findWorkspaceRoot();

  if (workspaceRoot === null) {
    log("Failed to find workspace root!");
    return null;
  }

  log("Workspace root", workspaceRoot);

  const config = getCurrentToolchain();
  const [cargo, cargoArgs] = cargoCommand(config);
  const isArgusInstalled = await checkVersionAndInstall(
    config,
    workspaceRoot,
    cargo,
    cargoArgs
  );

  if (!isArgusInstalled) {
    log("Argus failed to install");
    return null;
  }

  const argusOpts = await getCargoOpts(config, workspaceRoot, {
    RUST_LOG: "info",
    RUST_BACKTRACE: "1"
  });

  return <T extends ArgusCliOptions>(
    args: ArgusArgs<T>,
    noOutput = false,
    ignoreExitCode = false
  ): CPromise<ArgusResult<T>> => {
    log("Calling backend with args", args);
    const strArgs = _.map(args, arg => arg.toString());
    context.statusBar.setState("loading", "Waiting for Argus...");

    return execNotify(
      cargo,
      [...cargoArgs, "argus", ...strArgs],
      "Waiting for Argus...",
      {
        ...argusOpts,
        ignoreExitCode
      }
    )
      .then(output => {
        log("Received output from Argus");
        if (noOutput) {
          context.statusBar.setState("idle", "Argus is ready");
          return {
            type: "output",
            value: undefined as any
          } as ArgusResult<T>;
        }

        const outputTyped: Result<ArgusReturn<T>> = JSON.parse(output);
        log("Output Typed", outputTyped);
        if ("Err" in outputTyped) {
          log("Argus failed with `Err`", outputTyped.Err);
          context.statusBar.setState("error", "Analysis failed");
          context.extCtx.workspaceState.update(
            ARGUS_ERR_LOG_KEY,
            outputTyped.Err
          );
          return outputTyped.Err;
        }

        context.statusBar.setState("idle", "Argus is ready");
        return {
          type: "output",
          value: outputTyped.Ok
        } as ArgusResult<T>;
      })
      .catch(e => {
        context.statusBar.setState("error", "Build failure");
        context.extCtx.workspaceState.update(ARGUS_ERR_LOG_KEY, e);
        return {
          type: "build-error",
          error: e.toString()
        } as ArgusResult<T>;
      });
  };
}
