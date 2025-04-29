import fs from "node:fs";
import { resolve } from "node:path";
import { defineConfig } from "vite";
import dts from "vite-plugin-dts";

const manifest = JSON.parse(fs.readFileSync("package.json", "utf-8"));
const external = Object.keys(manifest.dependencies || {}).concat(["vscode"]);

export default defineConfig(({ mode }) => ({
  plugins: [
    dts({
      outDir: "dist/types",
      rollupTypes: true
    })
  ],
  build: {
    lib: {
      entry: resolve(__dirname, "src/lib.tsx"),
      name: "Panoptes",
      formats: ["es"]
    },
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      external
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
