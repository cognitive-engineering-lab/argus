{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
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

        depotjs = pkgs.rustPlatform.buildRustPackage rec {
          pname = "depot";
          version = "0.2.17";

          # Depot tests require lots of external toolchains, node, typedoc, biome, ...
          # so we'll just skip all tests for now and figure this out later.
          doCheck = false;

          src = pkgs.fetchFromGitHub {
            owner = "cognitive-engineering-lab";
            repo = pname;
            rev = "v${version}";
            hash = "sha256-kiQXxTVvzfovCn0YmOH/vTUQHyRS39gH7iBGaKyRZFg=";
          };

          cargoHash = "sha256-m9sG//vBUqGLwWHkyq+sJ8rkQOeaif56l394dgPU1uQ=";
          buildInputs = with pkgs; lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];
        };

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
          cargo fmt --check
          cargo clippy
          codespell .
          cargo test
        '';
      in {
        devShell = pkgs.mkShell {
          buildInputs = [ checkProject ] ++ (with pkgs; [
            llvmPackages_latest.llvm
            llvmPackages_latest.lld

            toolchain

            guile
            depotjs
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
            # Libraries needed in testing
            alsa-lib.dev
            udev.dev
          ]);

          RUSTC_LINKER = "${pkgs.llvmPackages.clangUseLLVM}/bin/clang";
        };

        # packages = rec {
        #   default = cargo-argus;

        #   cargo-argus = pkgs.rustPlatform.buildRustPackage {
        #     pname = name;
        #     inherit version;
        #     src = ./.;
        #     cargoSha256 = pkgs.lib.fakeHash;
        #     release = true;
        #   };

        #   # TODO package and release tutorial with nix
        #   # argus-tutorial = {};

        #   # TODO package and release extension with nix
        #   # vscode-argus = {};
        # };
      });
}
