import fs from "node:fs";
import { defineConfig } from "vite";
import { builtinModules } from "node:module";
import { resolve } from "node:path";

const manifest = JSON.parse(fs.readFileSync("package.json", "utf-8"));
export default defineConfig(({ mode }) => ({
  build: {
    lib: {
      entry: resolve(__dirname, "src/main.ts"),
      formats: ["cjs"]
    },
    minify: false,
    rollupOptions: {
      external: Object.keys(manifest.dependencies || {})
        .concat(builtinModules)
        .concat(builtinModules.map(s => `node:${s}`))
    }
  },
  define: {
    "process.env.NODE_ENV": JSON.stringify(mode)
  },
  test: {
    environment: "node",
    deps: {
      inline: [/^(?!.*vitest).*$/]
    }
  },
  resolve: { conditions: ["node"] }
}));
