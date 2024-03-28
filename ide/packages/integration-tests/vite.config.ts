import fs from "fs";
import { builtinModules } from "module";
import { resolve } from "path";
import { defineConfig } from "vite";

let manifest = JSON.parse(fs.readFileSync("package.json", "utf-8"));
export default defineConfig(({ mode }) => ({
  build: {
    lib: {
      entry: resolve(__dirname, "src/index.ts"),
      formats: ["cjs"],
    },
    minify: false,
    rollupOptions: {
      external: Object.keys(manifest.dependencies || {})
        .concat(builtinModules)
        .concat(["vscode"]),
    },
  },
  define: {
    "process.env.NODE_ENV": JSON.stringify(mode),
  },
  test: {
    environment: "node",
    deps: {
      inline: [/^(?!.*vitest).*$/],
    },
  },
  resolve: { conditions: ["node"] },
}));
