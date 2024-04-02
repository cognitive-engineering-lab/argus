import fs from "fs";
import { defineConfig } from "vite";
import { builtinModules } from "module";
import { resolve } from "path";

let manifest = JSON.parse(fs.readFileSync("package.json", "utf-8"));
export default defineConfig(({ mode }) => ({
  build: {
    lib: {
      entry: resolve(__dirname, "src/main.ts"),  
      formats: ["cjs"],
    },
    minify: false,
    rollupOptions: {
      external: Object.keys(manifest.dependencies || {}).concat(builtinModules)
    }
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
  resolve: {conditions: ["node"]},
}));
