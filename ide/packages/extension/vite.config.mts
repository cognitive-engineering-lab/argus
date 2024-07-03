import fs from "node:fs";
import { builtinModules } from "node:module";
import path, { resolve } from "node:path";
import toml from "toml";
import { defineConfig } from "vite";
import { viteStaticCopy } from "vite-plugin-static-copy";

const manifest = JSON.parse(fs.readFileSync("package.json", "utf-8"));
const rustToolchain = toml.parse(
  fs.readFileSync("../../../rust-toolchain.toml", "utf-8")
);
export default defineConfig(({ mode }) => ({
  plugins: [
    viteStaticCopy({
      targets: [
        {
          src: `${path.resolve(__dirname, "./node_modules/@argus")}/[!.]*`,
          dest: "./"
        }
      ]
    })
  ],
  build: {
    target: "node16",
    lib: {
      entry: resolve(__dirname, "src/main.ts"),
      name: "Extension",
      formats: ["cjs"]
    },
    rollupOptions: {
      external: Object.keys(manifest.dependencies || {})
        .concat(builtinModules)
        .concat(builtinModules.map(s => `node:${s}`))
        .concat(["vscode"])
    }
  },
  define: {
    "process.env.NODE_ENV": JSON.stringify(mode),
    TOOLCHAIN: JSON.stringify(rustToolchain.toolchain),
    VERSION: JSON.stringify(require("./package.json").version)
  },
  test: {
    environment: "jsdom",
    deps: {
      inline: [/^(?!.*vitest).*$/]
    }
  }
}));
