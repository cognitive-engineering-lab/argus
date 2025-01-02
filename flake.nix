{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    depot-js.url = "github:cognitive-engineering-lab/depot";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, depot-js }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        entry-crate = ./crates/argus-cli;
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        meta = (builtins.fromTOML (builtins.readFile (entry-crate/Cargo.toml))).package;
        inherit (meta) name version;

        mdbook-image-size = with pkgs; rustPlatform.buildRustPackage rec {
          pname = "mdbook-image-size";
          version = "0.2.0";

          src = fetchFromGitHub {
            owner = "lhybdv";
            repo = pname;
            rev = version;
            hash = "sha256-fySGDx3vbLsc3fL/54nMVjVRHNlQ2ZYSM4LMDHxUUvs=";
          };
          cargoHash = "sha256-iOTIjZr7vyduGTzK0xUssCKBKc8O0AYLpSdcozKPF2o=";
          doCheck = false;
        };

        checkProject = pkgs.writeScriptBin "ci-check" ''
          cargo fmt --check &&
          cargo clippy &&
          codespell . &&
          cargo test
        '';

        publishCrates = pkgs.writeScriptBin "ci-crate-pub" ''
          cargo ws publish skip --no-remove-dev-deps --from-git --yes --token "$1"
        '';

        publishExtension = pkgs.writeScriptBin "ci-ext-pub" ''
          cargo make init-bindings
          cd ide
          depot setup
          cd packages/extension
          vsce package
          vsce publish -p "$1" --packagePath argus-*.vsix
          pnpx ovsx publish argus-*.vsix -p "$2"
        '';
      in {
        devShell = with pkgs; mkShell {
          nativeBuildInputs = [ pkg-config ];
          buildInputs = [
            checkProject
            publishCrates
            publishExtension

            llvmPackages_latest.llvm
            llvmPackages_latest.lld

            toolchain

            # Required for the evaluation
            guile
            guile-json

            depot-js.packages.${system}.default
            nodejs_22
            nodePackages.pnpm
            codespell

            cargo-make
            cargo-watch
            rust-analyzer

            mdbook
            mdbook-mermaid
            mdbook-admonish
            mdbook-image-size

            vsce
            cargo-workspaces
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.SystemConfiguration
          ] ++ lib.optionals stdenv.isLinux [
            alsa-lib.dev
            udev.dev
          ];

          # Needed in order to run `cargo argus ...` within the directory
          shellHook = ''
            export DYLD_LIBRARY_PATH="$DYLD_LIBRARY_PATH:$(rustc --print target-libdir)"
          '';

          RUSTC_LINKER = "${llvmPackages.clangUseLLVM}/bin/clang";

          # NOTE: currently playwright-driver uses version 1.40.0, when something inevitably fails,
          # check that the version of playwright-driver and that of the NPM playwright 
          # `packages/evaluation/package.json` match.
          PLAYWRIGHT_BROWSERS_PATH="${playwright-driver.browsers}";
        };
      });
}
