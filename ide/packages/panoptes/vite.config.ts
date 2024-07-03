import fs from "node:fs";
import { resolve } from "node:path";
import { defineConfig } from "vite";

const manifest = JSON.parse(fs.readFileSync("package.json", "utf-8"));
export default defineConfig(({ mode }) => ({
  build: {
    lib: {
      entry: resolve(__dirname, "src/main.tsx"),
      name: "Panoptes",
      formats: ["iife"]
    },
    rollupOptions: {
      external: Object.keys(manifest.dependencies || {}).concat(["vscode"])
    }
  },
  define: {
    "process.env.NODE_ENV": JSON.stringify(mode)
  },
  test: {
    environment: "jsdom",
    deps: {
      inline: [/^(?!.*vitest).*$/]
    }
  }
}));
