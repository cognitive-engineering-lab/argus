import cp from "child_process";
import os from "os";
import path from "path";

export const LIBRARY_PATHS: Partial<Record<NodeJS.Platform, string>> = {
  darwin: "DYLD_LIBRARY_PATH",
  win32: "LIB",
};

export interface RustcToolchain {
  version: string;
  channel: string;
  components: string[];
}

export const cargoBin = () => {
  const cargo_home =
    process.env.CARGO_HOME || path.join(os.homedir(), ".cargo");
  return path.join(cargo_home, "bin");
};

export const cargoCommand = (config: RustcToolchain): [string, string[]] => {
  const cargo = "cargo";
  const toolchain = `+${config.channel}`;
  return [cargo, [toolchain]];
};

export type ExecNotifyOpts = {
  ignoreExitCode?: boolean;
  title?: string;
} & cp.SpawnOptionsWithoutStdio;

export const execNotifyBinary = async (
  log: (...args: any[]) => void,
  stateListener: (state: string) => void,
  cmd: string,
  args: string[],
  opts?: ExecNotifyOpts
): Promise<Buffer> => {
  log("Running command: ", cmd, args, opts);

  const proc = cp.spawn(cmd, args, opts ?? {});

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

  stateListener("loading");
  return new Promise<Buffer>((resolve, reject) => {
    proc.addListener("close", _ => {
      stateListener("idle");
      if (opts?.ignoreExitCode || proc.exitCode === 0) {
        resolve(Buffer.concat(stdoutChunks));
      } else {
        reject(stderrChunks.join(""));
      }
    });

    proc.addListener("error", e => {
      reject(e.toString());
    });
  });
};

export async function runInDir<T>(dir: string, thunk: () => Promise<T>) {
  const cd = process.cwd();
  try {
    process.chdir(dir);
    return await thunk();
  } finally {
    process.chdir(cd);
  }
}

export async function execNotify(
  cmd: string,
  args: string[],
  opts?: ExecNotifyOpts,
  log: (...args: any[]) => void = console.debug,
  stateListener: (state: string) => void = (..._args: any[]) => {}
): Promise<string> {
  const buffer = await execNotifyBinary(log, stateListener, cmd, args, opts);
  const text = buffer.toString("utf8");
  return text.trimEnd();
}

export async function getCargoOpts(
  config: RustcToolchain,
  cwd: string,
  additionalOpts: NodeJS.ProcessEnv = {}
) {
  const rustcPath = await execNotify(
    "rustup",
    ["which", "--toolchain", config.channel, "rustc"],
    {
      title: "Waiting for rustc...",
    }
  );

  const targetInfo = await execNotify(
    rustcPath,
    ["--print", "target-libdir", "--print", "sysroot"],
    {
      title: "Waiting for rustc...",
    }
  );

  const [targetLibdir, sysroot] = targetInfo.split("\n");
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
      PATH,
      ...additionalOpts,
      ...process.env,
    },
  };

  return opts;
}
