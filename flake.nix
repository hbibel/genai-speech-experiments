{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };
  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

      in
      with pkgs;
      {
        devShells.default = mkShell {
          buildInputs = [
            rustToolchain

            # Tools for C bindings
            pkg-config
            libclang
            # Pipewire C libraries and their headers
            pipewire.dev

            alsa-lib.dev

            ffmpeg_6
          ];

          shellHook = ''
            export LIBCLANG_PATH="${libclang.lib}/lib"

            CLANG_VERSION=$(ls ${libclang.lib}/lib/clang/ | head -1)

            # The pipewire Rust package uses bindgen and clang
            export BINDGEN_EXTRA_CLANG_ARGS="-I${libclang.lib}/lib/clang/$CLANG_VERSION/include"
          '';

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        };
      }
    );
}
