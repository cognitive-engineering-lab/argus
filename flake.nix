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
    toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
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
  in {
    devShell = with pkgs; mkShell { 
      buildInputs = [ 
        # Deployment only
        vsce
        cargo-workspaces

        llvmPackages_latest.llvm
        llvmPackages_latest.lld

        guile
        depotjs
        nodejs_22
        nodePackages.pnpm

        mdbook
        cargo-make
        cargo-watch
        rust-analyzer

        toolchain
      ] ++ lib.optionals stdenv.isDarwin [
        darwin.apple_sdk.frameworks.SystemConfiguration
      ];    

      RUSTC_LINKER = "${pkgs.llvmPackages.clangUseLLVM}/bin/clang";
    };
  });
}
