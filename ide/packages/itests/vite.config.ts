import fs from "fs";
import { builtinModules } from "module";
import path from "path";
import { defineConfig } from "vite";

let tests = fs.readdirSync("src").map(p => path.join("src", p));
let manifest = JSON.parse(fs.readFileSync("package.json", "utf-8"));
export default defineConfig(({ mode }) => ({
  build: {
    lib: {
      entry: tests,
      formats: ["cjs"],
    },
    minify: false,
    rollupOptions: {
      external: Object.keys(manifest.dependencies || {}).concat(builtinModules),
    },
  },
  define: {
    "process.env.NODE_ENV": JSON.stringify(mode),
  },
  resolve: { conditions: ["node"] },
}));
