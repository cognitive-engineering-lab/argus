import { CancelablePromise as CPromise } from "cancelable-promise";
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

export async function runInDir<T>(dir: string, thunk: () => Promise<T>) {
  const cd = process.cwd();
  try {
    process.chdir(dir);
    return thunk();
  } finally {
    process.chdir(cd);
  }
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

export function killAll(
  pid: number,
  signal: string | number = "SIGTERM",
  logger: (...args: any[]) => void = console.debug
) {
  if (process.platform == "win32") {
    cp.exec(`taskkill /PID ${pid} /T /F`, (error, stdout, stderr) => {
      logger("taskkill stdout: " + stdout);
      logger("taskkill stderr: " + stderr);
      if (error) {
        logger("error: " + error.message);
      }
    });
  } else {
    // NOTE: calling this usually throws the error 'ESRCH', meaning that
    // the given pid isn't running. However, this is also the only solution
    // that killds the entire process family, so I'm not sure where the error
    // is coming from.
    process.kill(-pid, signal);
  }
}

export const execNotifyBinary = (
  log: (...args: any[]) => void,
  stateListener: (state: string) => void,
  cmd: string,
  args: string[],
  opts?: ExecNotifyOpts
): CPromise<Buffer> => {
  const msg = (...args: any[]) => {
    log(...args);
    console.debug(...args);
  };

  const proc = cp.spawn(cmd, args, { ...opts, detached: true });
  msg(`process ${proc.pid}, command: `, cmd, args, opts);

  let stdoutChunks: Buffer[] = [];
  proc.stdout.on("data", data => {
    stdoutChunks.push(data);
  });

  let stderrChunks: string[] = [];
  proc.stderr.setEncoding("utf8");
  proc.stderr.on("data", data => {
    msg(data);
    stderrChunks.push(data);
  });

  stateListener("loading");
  const killProcess = () => {
    try {
      msg(`Killing process ${proc.pid}`);
      killAll(proc.pid!, "SIGKILL", msg);
    } catch (e: any) {
      log(`Error killing process ${proc.pid}: ${e.toString()}`);
    }
  };
  const peacefulCancel = () => {};

  return new CPromise<Buffer>((resolve, reject, onCancel) => {
    onCancel(killProcess);

    proc.addListener("close", _ => {
      onCancel(peacefulCancel);
      stateListener("idle");
      if (opts?.ignoreExitCode || proc.exitCode === 0) {
        resolve(Buffer.concat(stdoutChunks));
      } else {
        reject(stderrChunks.join(""));
      }
    });

    proc.addListener("error", e => {
      onCancel(peacefulCancel);
      stateListener("error");
      reject(e.toString());
    });
  });
};

export function execNotify(
  cmd: string,
  args: string[],
  opts?: ExecNotifyOpts,
  log: (...args: any[]) => void = console.debug,
  stateListener: (state: string) => void = (..._args: any[]) => {}
): CPromise<string> {
  return execNotifyBinary(log, stateListener, cmd, args, opts).then(buffer => {
    const text = buffer.toString("utf8");
    return text.trimEnd();
  });
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
