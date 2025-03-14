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
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        meta = (builtins.fromTOML (builtins.readFile (./crates/argus-cli/Cargo.toml))).package;
        inherit (meta) name version;

        rustNightly = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };

        mdbook-image-size = rustNightly.buildRustPackage rec {
          pname = "mdbook-image-size";
          version = "0.2.0";

          src = pkgs.fetchFromGitHub {
            owner = "lhybdv";
            repo = pname;
            rev = version;
            hash = "sha256-fySGDx3vbLsc3fL/54nMVjVRHNlQ2ZYSM4LMDHxUUvs=";
          };
          cargoHash = "sha256-qeak0f6mVcIgMQrh8a6R+p7KzSTdiBE4meBhCPyT2kk=";
          doCheck = false;
        };

        env-vars = {
          RUSTC_LINKER = "${pkgs.llvmPackages.clangUseLLVM}/bin/clang";
          # NOTE: currently playwright-driver uses version 1.40.0, when something inevitably fails,
          # check that the version of playwright-driver and that of the NPM playwright
          # `packages/evaluation/package.json` match.
          PLAYWRIGHT_BROWSERS_PATH="${pkgs.playwright-driver.browsers}";
        };

        native-deps = with pkgs; [
          pkg-config
          cacert
        ] ++ lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.SystemConfiguration
        ];

        cli-deps = with pkgs; [
          llvmPackages_latest.llvm
          llvmPackages_latest.lld
          toolchain
          guile
          guile-json
          codespell
          cargo-make
          cargo-watch
        ];

        ide-deps = with pkgs; [
          depot-js.packages.${system}.default
          nodejs_20
          pnpm_9
          biome
          vsce
        ];

        book-deps = with pkgs; [
          mdbook
          mdbook-mermaid
          mdbook-admonish
          mdbook-image-size
        ];

        argus-cli = rustNightly.buildRustPackage {
          pname = name;
          inherit version;
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = native-deps;
          buildInputs = cli-deps;
          doCheck = false;
          env = env-vars;
        };


        archiveBase = "${name}-${version}";
        vscodeExtPublisher = "gavinleroy";
        packageArgusWithExt = ext: ''
          cargo make init-bindings
          cd ide/packages/extension
          vsce package -o ${archiveBase}.${ext}
        '';

        argus-vsix = pkgs.stdenv.mkDerivation {
          name = "argus-vsix";
          inherit version;
          src = pkgs.lib.cleanSource ./.;
          nativeBuildInputs = native-deps;
          buildInputs = cli-deps ++ ide-deps; 

          env = (env-vars // {
            CARGO_HOME = "${placeholder "out"}/.cargo";
          });

          buildPhase = packageArgusWithExt "zip";
          installPhase = ''
            mkdir -p $out/share/vscode/extensions
            mv ${archiveBase}.zip $out/share/vscode/extensions/
          '';
        };

        argus-ide = pkgs.vscode-utils.buildVscodeExtension rec {
          inherit name version vscodeExtPublisher;
          src = "${argus-vsix}/share/vscode/extensions/${archiveBase}.zip";
          vscodeExtName = name;
          vscodeExtUniqueId = "${vscodeExtPublisher}.${name}";
        };

        argus-book = pkgs.stdenv.mkDerivation {
          name = "argus-book";
          inherit version;
          src = pkgs.lib.cleanSource ./book;
          buildInputs = book-deps;

          buildPhase = ''
            mdbook-admonish install .
            mdbook-mermaid install .
            mdbook build
          '';

          installPhase = ''
            mkdir -p $out
            cp -r book/* $out
          '';
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

        publishExtension = pkgs.writeScriptBin "ci-ext-pub" ((packageArgusWithExt "vsix") + ''
          vsce publish -p "$1" --packagePath ${archiveBase}.vsix
          pnpx ovsx publish ${archiveBase}.vsix -p "$2"
        '');
      in {
        packages = { inherit argus-cli argus-ide argus-book; };
        devShell = with pkgs; mkShell ({
          nativeBuildInputs = native-deps;
          buildInputs = cli-deps ++ ide-deps ++ book-deps ++ [
            checkProject
            publishCrates
            publishExtension
            cargo-workspaces
            rust-analyzer
          ] ++ lib.optionals stdenv.isLinux [
            alsa-lib.dev
            udev.dev
          ];

          # Needed in order to run `cargo argus ...` within the directory
          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:$(rustc --print target-libdir)"
            export DYLD_LIBRARY_PATH="$DYLD_LIBRARY_PATH:$(rustc --print target-libdir)"
          '';
        } // env-vars);
      });
}
