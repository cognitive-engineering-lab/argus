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
          #SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
        };

        shell-env = {
          # NOTE: The version of playwright-driver and that of the NPM playwright
          # `packages/evaluation/package.json` must match.
          PLAYWRIGHT_BROWSERS_PATH="${pkgs.playwright-driver.browsers}";
        } // pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          PKG_CONFIG_PATH="${pkgs.udev.dev}/lib/pkgconfig:${pkgs.alsa-lib.dev}/lib/pkgconfig";
        } // pkgs.lib.optionalAttrs pkgs.stdenv.isDarwin {
          RUSTC_LINKER = "${pkgs.llvmPackages.clangUseLLVM}/bin/clang";
        };

        native-deps = with pkgs; [
          # pkg-config
          # cacert
        ];

        cli-deps = with pkgs; [
          toolchain
          guile
          codespell
          cargo-make
          cargo-watch
        ] ++ lib.optionals stdenv.isDarwin [
          llvmPackages.llvm
          llvmPackages.lld
        ];

        pnpm = pkgs.pnpm_9;
        nodejs = pkgs.nodejs_22;
        ide-deps = [
          nodejs
          pnpm
          depot-js.packages.${system}.default
          pkgs.biome
          pkgs.vsce
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
            CARGO_HOME="${placeholder "out"}/.cargo";
          });

          postBuild = ''
            cargo make init-bindings
          '';

          postInstall = ''
            mkdir -p $out/lib
            cp ide/packages/common/src/bindings.ts $out/lib/
          '';
        };

        # FIXME we need to manage the pnpm deps with Nix
        archiveBase = "argus-${version}";
        packageArgusWithExt = ext: ''
          cp ${argus-cli}/lib/bindings.ts ide/packages/common/src/bindings.ts
          cd ide/packages/extension
          vsce package --allow-unused-files-pattern -o ${archiveBase}.${ext}
        '';

        pnpmDepsHashes = {
          "x86_64-linux" = "sha256-sjWzbU2/nZ4A+n0wd/4kYJkrKTl1dWTr0ZEIoSegBwU=";
          "aarch64-linux" = "sha256-sJSoHA3kWOMSbV1TsB8kncWu96ocTAXnQLKG6NfvTLk=";
          #"aarch64-darwin" = "sha256-j364V5JhDS78fy6hzQPDbzhzG/s0ERe8dL0zc7hzwhE=";
          "aarch64-darwin" = "sha256-sjWzbU2/nZ4A+n0wd/4kYJkrKTl1dWTr0ZEIoSegBwU=";
        };

        argus-ide = pkgs.stdenv.mkDerivation (finalAttrs: {
          pname = "argus-ide";
          inherit version;
          src = pkgs.lib.cleanSource ./.;
          nativeBuildInputs = native-deps ++ ide-deps ++ [
            pnpm.configHook
            argus-cli
          ];
          env = env-vars;

          pnpmRoot = "ide";
          pnpmWorkspaces = [
            "@argus/common"
            "@argus/evaluation"
            "@argus/itests"
            "@argus/panoptes"
            "@argus/print"
            "@argus/system"
            "argus" # The extension
          ];
          pnpmDeps = pnpm.fetchDeps {
            inherit (finalAttrs) pname version src pnpmWorkspaces;
            hash = pnpmDepsHashes.${system};
            sourceRoot = "${finalAttrs.src.name}/ide";
          };

          buildPhase = packageArgusWithExt "zip";
          installPhase = ''
            mkdir -p $out/share/vscode/extensions
            mkdir -p $out/packages
            mv ${archiveBase}.zip $out/share/vscode/extensions/
            cd ../
            cp -LR evaluation $out/packages/evaluation 
            cp -LR panoptes $out/packages/panoptes 
          '';
        });

        argus-extension = pkgs.vscode-utils.buildVscodeExtension rec {
          name = "argus-ide";
          inherit version;
          nativeBuildInputs = [ argus-ide ];
          vscodeExtPublisher = "gavinleroy";
          src = "${argus-ide}/share/vscode/extensions/${archiveBase}.zip";
          vscodeExtName = name;
          vscodeExtUniqueId = "gavinleroy.argus";
        };

        argus-book = pkgs.stdenv.mkDerivation {
          pname = "argus-book";
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

        ci-check = pkgs.writeScriptBin "ci-check" ''
          cargo fmt --check &&
          cargo clippy -- -D warnings &&
          codespell . &&
          cargo test
        '';

        publishCrates = pkgs.writeScriptBin "ci-crate-pub" ''
          cargo ws publish skip --no-remove-dev-deps --from-git --yes --token "$1"
        '';

        publishExtension = pkgs.writeScriptBin "ci-ext-pub" ((packageArgusWithExt "vsix") + ''
          vsce publish -p "$1" --packagePath ${archiveBase}.vsix &&
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

        devShells.default = pkgs.mkShell ({
          nativeBuildInputs = native-deps;
          buildInputs = cli-deps ++ ide-deps ++ book-deps ++ [
            pkgs.rust-analyzer
            pkgs.playwright-driver.browsers
            ci-check
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.alsa-lib.dev
            pkgs.udev.dev
          ];

          # Needed in order to run `cargo argus ...` within the directory
          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:$(rustc --print target-libdir)"
            export DYLD_LIBRARY_PATH="$DYLD_LIBRARY_PATH:$(rustc --print target-libdir)"
          '';
        } // env-vars // shell-env);

        devShells.ci = pkgs.mkShell ({
          nativeBuildInputs = native-deps;
          buildInputs = cli-deps ++ ide-deps ++ book-deps ++ [
            pkgs.cargo-workspaces
            pkgs.playwright-driver.browsers
            ci-check
            publishCrates
            publishExtension
          ];
        } // env-vars // shell-env);
      });
}
