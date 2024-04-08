import { defineConfig } from "@vscode/test-cli";
import fs from "fs";
import path from "path";

let workspaces = fs
  .readdirSync("workspaces")
  .map(p => path.join("workspaces", p));

let configs = workspaces.map(ws => ({
  files: "dist/*.cjs",
  workspaceFolder: ws,
}));

export default defineConfig(configs);
