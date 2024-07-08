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
  in {
    devShell = with pkgs; mkShell { 
      buildInputs = [ 
        llvmPackages_latest.llvm
        llvmPackages_latest.lld

        guile
        nodejs_22
        nodePackages.pnpm
        toolchain
      ] ++ lib.optional stdenv.isDarwin libiconv; 

      # FIXME: this is darwin specific vvv but the flake should work for all systems
      RUSTC_LINKER = "${pkgs.llvmPackages.clangUseLLVM}/bin/clang";
      RUSTFLAGS = "-Clink-arg=-fuse-ld=${pkgs.mold}/bin/mold";
    };
  });
}
