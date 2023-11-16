import * as cp from "child_process";
import * as _ from "lodash";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";

// // import { download } from "./download";
import { ArgusError, ArgusResult, showErrorDialog } from "./errors";
import { globals } from "./lib";
import { log } from "./logging";

// FIXME: this file is a wreck...somewhere there is a hardcoded path to the
// argus directory. Running the tool causes a recompile of argus (not sure why).

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

// serde-compatible type
type Result<T> = { Ok: T } | { Err: ArgusError };

// Quick hack to avoid needing to figure out node.
const HARD_PATH: string = "/Users/gavinleroy/dev/prj/argus";

const LIBRARY_PATHS: Partial<Record<NodeJS.Platform, string>> = {
  darwin: "DYLD_LIBRARY_PATH",
  win32: "LIB",
};

export const getOpts = async (cwd: string) => {
  const rustcPath = await execNotify(
    "rustup",
    ["which", "--toolchain", TOOLCHAIN.channel, "rustc"],
    "Waiting for rustc..."
  );
  const targetInfo = await execNotify(
    rustcPath,
    ["--print", "target-libdir", "--print", "sysroot"],
    "Waiting for rustc..."
  );

  const [targetLibdir, sysroot] = targetInfo.split("\n");
  log("Target libdir:", targetLibdir);
  log("Sysroot: ", sysroot);

  const library_path = LIBRARY_PATHS[process.platform] || "LD_LIBRARY_PATH";

  const PATH = cargoBin() + ";" + process.env.PATH;

  return {
    cwd,
    [library_path]: targetLibdir,
    SYSROOT: sysroot,
    RUST_BACKTRACE: "1",
    PATH,
  };
};

let execNotifyBinary = async (
  cmd: string,
  args: string[],
  _title: string,
  opts?: any
): Promise<Buffer> => {
  log("Running command: ", [cmd, ...args].join(" "));
  console.warn("Running command: ", [cmd, ...args].join(" "));

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
      // globals.status_bar.set_state("idle");
      if (proc.exitCode !== 0) {
        reject(stderrChunks.join(""));
      } else {
        resolve(Buffer.concat(stdoutChunks));
      }
    });
    proc.addListener("error", e => {
      // globals.status_bar.set_state("idle");
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

export type CallArgus = <T>(
  _args: string[],
  _no_output?: boolean
) => Promise<ArgusResult<T>>;

export let debuggerManifest = (): string => {
  let base = process.env.TDBG_PATH || HARD_PATH;
  return `${base}/Cargo.toml`;
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
  let workspaceRoot = await findWorkspaceRoot();
  let dbgRoot = debuggerManifest();
  let rktFmtRoot = `${dbgRoot}/scripts/imgfmt.rkt`;

  if (dbgRoot == null) {
    log("Failed to find debugger root!");
    return null;
  }

  if (workspaceRoot === null) {
    log("Failed to find workspace root!");
    return null;
  }

  log("Workspace root", workspaceRoot);
  log("Trait Debugger", dbgRoot);
  let [cargo, cargoArgs] = cargoCommand();

  return async <T>(args: string[], noOutput: boolean = false) => {
    let output;
    try {
      let editor = vscode.window.activeTextEditor;
      if (editor) {
        await editor.document.save();
      }

      // FIXME this assumes that the `cargo_cli` binary is installed, 
      // and that everything is just going to "work out".
      output = await execNotifyBinary(
        cargo,
        [
          ...cargoArgs,
          "argus",
          ...args,
        ],
        "Waiting for Argus...",
        {
          env: {
            cwd: workspaceRoot,
            ...process.env,
          },
        }
      );
    } catch (e: any) {
      context.workspaceState.update("err_log", e);
      return {
        type: "BuildError",
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
      let outputBytes = Buffer.from(output.toString("utf8"), "base64");
      let outputStr = output.toString("utf8"); // outputBytes.toString('utf8');
      console.warn(outputStr);
      outputTyped = JSON.parse(outputStr);
    } catch (e: any) {
      return {
        type: "AnalysisError",
        error: e.toString(),
      };
    }

    return {
      type: "output",
      value: outputTyped,
    };
  };
}