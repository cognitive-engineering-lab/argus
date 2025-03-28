{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/80a3e9ca766a82fcec24648ab3a771d5dd8f9bf2";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    depot-js.url = "github:cognitive-engineering-lab/depot?rev=3676b134767aba6a951ed5fdaa9e037255921475";
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
          cargoHash = "sha256-TPtBuabdx80Lgjn8s2CNPXfN6DdMJTI230Vm09exX9A=";
          doCheck = false;
          useFetchCargoVendor = true;
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
          nativeBuildInputs = native-deps ++ cli-deps;
          useFetchCargoVendor = true;
          doCheck = false;

          env = (env-vars // {
            CARGO_HOME = "${placeholder "out"}/.cargo";
          });

          postBuild = ''
            cargo make init-bindings
          '';

          postInstall = ''
            mkdir -p $out/lib
            cp ide/packages/common/src/bindings.ts $out/lib/
          '';
        };

        archiveBase = "argus-${version}";
        packageArgusWithExt = ext: ''
          cp ${argus-cli}/lib/bindings.ts ide/packages/common/src/bindings.ts
          cd ide/packages/extension
          vsce package --allow-unused-files-pattern -o ${archiveBase}.${ext}
        '';

        argus-ide = pkgs.stdenv.mkDerivation {
          name = "argus-ide";
          inherit version;
          src = pkgs.lib.cleanSource ./.;
          nativeBuildInputs = native-deps ++ ide-deps;
          env = env-vars;
          buildPhase = packageArgusWithExt "zip";
          installPhase = ''
            mkdir -p $out/share/vscode/extensions
            mv ${archiveBase}.zip $out/share/vscode/extensions/
            cd ../../../
            cp -LR ide $out/lib
          '';
        };

        argus-extension = pkgs.vscode-utils.buildVscodeExtension rec {
          name = "argus-ide";
          vscodeExtPublisher = "gavinleroy";
          inherit version;
          src = "${argus-ide}/share/vscode/extensions/${archiveBase}.zip";
          vscodeExtName = name;
          vscodeExtUniqueId = "gavinleroy.argus";
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
            cp -R book/* $out
          '';
        };

        checkProject = pkgs.writeScriptBin "ci-check" ''
          cargo fmt --check
          cargo clippy
          codespell .
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
        packages = { 
          inherit 
          argus-cli 
          argus-ide 
          argus-extension 
          argus-book;
        };

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
