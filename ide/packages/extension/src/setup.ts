import cp from "child_process";
import os from "os";
import path from "path";
import vscode from "vscode";
import _ from "lodash";

import { ArgusArgs, CallArgus, Result } from "@argus/common";
import { log } from "./logging";

// TODO: read the version from rust-toolchain.toml
declare const VERSION: string;

// TODO: read the version from rust-toolchain.toml
// declare const TOOLCHAIN: {
//   channel: string;
//   components: string[];
// };

const TOOLCHAIN = {
  channel: "stage1",
  components: ["rust-std", "rustc-dev", "llvm-tools-preview"],
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
      RUST_LOG: "debug", // TODO: remove this, turn this into a debugging option
      RUST_BACKTRACE: "1",
      PATH,
      ...process.env,
    },
  };

  log("Argus options:", opts);

  return opts;
};

let execNotifyBinary = async (
  cmd: string,
  args: string[],
  _title: string,
  opts?: any
): Promise<Buffer> => {

  log("Running command: ", cmd, args, opts);

  let proc = cp.spawn(cmd, args, opts);

  let stdoutChunks: Buffer[] = [];
  proc.stdout.on("data", data => {
    stdoutChunks.push(data);
  });

  let stderrChunks: string[] = [];
  proc.stderr.setEncoding("utf8");
  proc.stderr.on("data", data => {
    log(data);
    stderrChunks.push(data);
  });

  // globals.status_bar.set_state("loading", title);

  return new Promise<Buffer>((resolve, reject) => {
    proc.addListener("close", _ => {
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

export let execNotify = async (
  cmd: string,
  args: string[],
  title: string,
  opts?: any
): Promise<string> => {
  let buffer = await execNotifyBinary(cmd, args, title, opts);
  let text = buffer.toString("utf8");
  return text.trimEnd();
};

export let cargoBin = () => {
  let cargo_home = process.env.CARGO_HOME || path.join(os.homedir(), ".cargo");
  return path.join(cargo_home, "bin");
};

export let cargoCommand = (): [string, string[]] => {
  let cargo = "cargo";
  let toolchain = `+${TOOLCHAIN.channel}`;
  return [cargo, [toolchain]];
};

let findWorkspaceRoot = async (): Promise<string | null> => {
  let folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) {
    log("No folders exist");
    return null;
  }

  let hasCargoToml = async (dir: string) => {
    let manifestPath = path.join(dir, "Cargo.toml");
    let manifestUri = vscode.Uri.file(manifestPath);
    try {
      await vscode.workspace.fs.stat(manifestUri);
      return true;
    } catch (e) {
      return false;
    }
  };

  let folderPath = folders[0].uri.fsPath;
  if (await hasCargoToml(folderPath)) return folderPath;

  let activeEditor = vscode.window.activeTextEditor;
  if (!activeEditor) {
    log("No active editor exists");
    return null;
  }

  let activeFilePath = activeEditor.document.fileName;
  log(`Looking for workspace root between ${folderPath} and ${activeFilePath}`);

  let components = path.relative(folderPath, activeFilePath).split(path.sep);
  let folderSubdirTil = (idx: number) =>
    path.join(folderPath, ...components.slice(0, idx));
  let prefixHasToml = await Promise.all(
    _.range(components.length).map(idx => ({
      idx,
      has: hasCargoToml(folderSubdirTil(idx)),
    }))
  );
  let entry = prefixHasToml.find(({ has }) => has);
  if (entry === undefined) return null;

  return folderSubdirTil(entry.idx);
};

export async function setup(
  context: vscode.ExtensionContext
): Promise<CallArgus | null> {
  log("Getting workspace root");

  let workspaceRoot = await findWorkspaceRoot();

  if (workspaceRoot === null) {
    log("Failed to find workspace root!");
    return null;
  }

  log("Workspace root", workspaceRoot);

  let [cargo, cargoArgs] = cargoCommand();

  let argusOpts = await getArgusOpts(workspaceRoot);

  return async <T>(args: ArgusArgs, noOutput: boolean = false) => {
    log("Calling backend with args", args);

    let output;
    try {
      let editor = vscode.window.activeTextEditor;

      if (editor) {
        await editor.document.save();
      }

      output = await execNotify(
        cargo,
        [ ...cargoArgs, "argus", ...args],
        "Waiting for Argus...",
        argusOpts
      );
    } catch (e: any) {
      context.workspaceState.update("err_log", e);
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

      let outputStr = output;
      outputTyped = JSON.parse(outputStr);
    } catch (e: any) {
      return {
        type: "analysis-error",
        error: e.toString(),
      };
    }

    if ("Err" in outputTyped) {
      return {
        type: "analysis-error",
        error: outputTyped.Err,
      }
    }

    return {
      type: "output",
      value: outputTyped.Ok,
    };
  };
}