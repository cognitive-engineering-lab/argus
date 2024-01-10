import fs from "fs";
import { builtinModules } from "module";
import { resolve } from "path";
import { defineConfig } from "vite";

let manifest = JSON.parse(fs.readFileSync("package.json", "utf-8"));
export default defineConfig(({ mode }) => ({
  build: {
    target: "node16",
    lib: {
      entry: resolve(__dirname, "src/main.ts"),
      name: "Extension",
      formats: ["cjs"],
    },
    rollupOptions: {
      external: Object.keys(manifest.dependencies || {}).concat(builtinModules),
    },
  },
  define: {
    "process.env.NODE_ENV": JSON.stringify(mode),
  },
  test: {
    environment: "jsdom",
    deps: {
      inline: [/^(?!.*vitest).*$/],
    },
  },
}));
