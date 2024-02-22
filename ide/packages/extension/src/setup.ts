import { ArgusArgs, CallArgus, Result } from "@argus/common/lib";
import cp from "child_process";
import _ from "lodash";
import open from "open";
import os from "os";
import path from "path";
import vscode from "vscode";

import { Ctx } from "./ctx";
import { log } from "./logging";
import { globals } from "./main";

declare const VERSION: string;
declare const TOOLCHAIN: {
  channel: string;
  components: string[];
};

const LIBRARY_PATHS: Partial<Record<NodeJS.Platform, string>> = {
  darwin: "DYLD_LIBRARY_PATH",
  win32: "LIB",
};

export const getArgusOpts = async (cwd: string) => {
  log("Getting Argus options");

  const rustcPath = await execNotify(
    "rustup",
    ["which", "--toolchain", TOOLCHAIN.channel, "rustc"],
    "Waiting for rustc..."
  );

  log("Rustc path:", rustcPath);

  const targetInfo = await execNotify(
    rustcPath,
    ["--print", "target-libdir", "--print", "sysroot"],
    "Waiting for rustc..."
  );

  const [targetLibdir, sysroot] = targetInfo.split("\n");

  log("Target libdir:", targetLibdir);
  log("Sysroot: ", sysroot);

  const libraryPath = LIBRARY_PATHS[process.platform] || "LD_LIBRARY_PATH";

  const PATH = cargoBin() + ";" + process.env.PATH;

  // For each element in libraryPath, we need to add the targetLibdir as its value.
  // This should then get added to the opts object.
  const opts = {
    cwd,
    env: {
      [libraryPath]: targetLibdir,
      LD_LIBRARY_PATH: targetLibdir,
      SYSROOT: sysroot,
      RUST_LOG: "info",
      RUST_BACKTRACE: "1",
      PATH,
      ...process.env,
    },
  };

  log("Argus options:", opts);

  return opts;
};

const execNotifyBinary = async (
  cmd: string,
  args: string[],
  title: string,
  opts?: any
): Promise<Buffer> => {
  log("Running command: ", cmd, args, opts);

  const proc = cp.spawn(cmd, args, opts);

  const stdoutChunks: Buffer[] = [];
  proc.stdout.on("data", data => {
    stdoutChunks.push(data);
  });

  const stderrChunks: string[] = [];
  proc.stderr.setEncoding("utf8");
  proc.stderr.on("data", data => {
    log(data);
    stderrChunks.push(data);
  });

  globals.statusBar.setState("loading", title);
  return new Promise<Buffer>((resolve, reject) => {
    proc.addListener("close", _ => {
      globals.statusBar.setState("idle");
      if (proc.exitCode !== 0) {
        reject(stderrChunks.join(""));
      } else {
        resolve(Buffer.concat(stdoutChunks));
      }
    });

    proc.addListener("error", e => {
      reject(e.toString());
    });
  });
};

export const execNotify = async (
  cmd: string,
  args: string[],
  title: string,
  opts?: any
): Promise<string> => {
  const buffer = await execNotifyBinary(cmd, args, title, opts);
  const text = buffer.toString("utf8");
  return text.trimEnd();
};

export const cargoBin = () => {
  const cargo_home =
    process.env.CARGO_HOME || path.join(os.homedir(), ".cargo");
  return path.join(cargo_home, "bin");
};

export const cargoCommand = (): [string, string[]] => {
  const cargo = "cargo";
  const toolchain = `+${TOOLCHAIN.channel}`;
  return [cargo, [toolchain]];
};

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
      has: hasCargoToml(folderSubdirTil(idx)),
    }))
  );
  const entry = prefixHasToml.find(({ has }) => has);
  if (entry === undefined) return null;

  return folderSubdirTil(entry.idx);
};

const checkVersionAndInstall = async (
  workspaceRoot: string,
  cargo: string,
  cargoArgs: string[]
): Promise<boolean> => {
  let version;
  try {
    version = await execNotify(
      cargo,
      [...cargoArgs, "argus", "-V"],
      "Waiting for Argus...",
      { cwd: workspaceRoot }
    );
  } catch (e) {
    version = "";
  }

  if (version !== VERSION) {
    log(
      `Argus binary version ${version} does not match expected IDE version ${VERSION}`
    );
    const components = TOOLCHAIN.components.map(c => ["-c", c]).flat();
    try {
      await execNotify(
        "rustup",
        [
          "toolchain",
          "install",
          TOOLCHAIN.channel,
          "--profile",
          "minimal",
          ...components,
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
          "https://github.com/gavinleroy/argus/blob/master/README.md#rustup-fails-on-install"
        );
        await vscode.window.showInformationMessage(
          'Click "Continue" once you have completed the fix.',
          "Continue"
        );
      } else {
        return false;
      }
    }

    await execNotify(
      cargo,
      [...cargoArgs, "install", "argus_cli", "--version", VERSION, "--force"],
      "Installing Argus from source... (this may take a minute)"
    );

    if (version === "") {
      vscode.window.showInformationMessage("Argus has successfully installed!");
    }
  }

  return true;
};

export async function setup(context: Ctx): Promise<CallArgus | null> {
  log("Getting workspace root");

  const workspaceRoot = await findWorkspaceRoot();

  if (workspaceRoot === null) {
    log("Failed to find workspace root!");
    return null;
  }

  log("Workspace root", workspaceRoot);

  const [cargo, cargoArgs] = cargoCommand();
  const isArgusInstalled = await checkVersionAndInstall(
    workspaceRoot,
    cargo,
    cargoArgs
  );

  if (!isArgusInstalled) {
    log("Argus failed to install");
    return null;
  }

  const argusOpts = await getArgusOpts(workspaceRoot);
  return async <T>(args: ArgusArgs, noOutput: boolean = false) => {
    log("Calling backend with args", args);

    let output;
    try {
      const editor = vscode.window.activeTextEditor;

      if (editor) {
        await editor.document.save();
      }
      const strArgs = _.map(args, arg => arg.toString());
      output = await execNotify(
        cargo,
        [...cargoArgs, "argus", ...strArgs],
        "Waiting for Argus...",
        argusOpts
      );
    } catch (e: any) {
      context.extCtx.workspaceState.update("err_log", e);
      return {
        type: "build-error",
        error: e,
      };
    }
    if (noOutput) {
      return {
        type: "output",
        value: undefined as any,
      };
    }

    let outputTyped: Result<T>;
    try {
      log("output", output);
      outputTyped = JSON.parse(output);
    } catch (e: any) {
      context.extCtx.workspaceState.update("err_log", e);
      return {
        type: "analysis-error",
        error: e.toString(),
      };
    }

    if ("Err" in outputTyped) {
      return {
        type: "analysis-error",
        error: outputTyped.Err,
      };
    }

    return {
      type: "output",
      value: outputTyped.Ok,
    };
  };
}
