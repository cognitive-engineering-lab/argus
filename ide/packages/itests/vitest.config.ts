/// <reference types="vitest" />
import fs from "fs";
import { defineConfig } from "vite";
import { builtinModules } from "module";

let manifest = JSON.parse(fs.readFileSync("package.json", "utf-8"));
export default defineConfig(({ mode }) => ({
  define: {
    "process.env.NODE_ENV": JSON.stringify(mode),
  },
  test: {
    environment: "node",
    deps: {
      inline: [/^(?!.*vitest).*$/],
    },
  },
  resolve: {conditions: ["node"]},
}));
