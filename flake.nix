{
  description = "Nix flake for flashcards";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, fenix, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system: {
      devShells.default =
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [
              (_: super: let pkgs = fenix.inputs.nixpkgs.legacyPackages.${super.system}; in fenix.overlays.default pkgs pkgs)
            ];
          };
          libraries = with pkgs; [
            stdenv.cc.cc
            openssl
            graphite2
            icu
            freetype
            fontconfig
            harfbuzz
            libpng
            zlib
          ];
          toolchain = fenix.packages."${system}".complete.toolchain;
        in
        pkgs.mkShell {
          nativeBuildInputs = (with pkgs; [
            pkg-config
            graphite2.dev
            icu
            freetype
            fontconfig
            harfbuzz
          ]) ++ [
            toolchain
          ];

          packages = [
            pkgs.cargo-shuttle
            pkgs.rust-analyzer-nightly
          ];

          buildInputs = libraries;
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libraries;
        };
    });

  nixConfig = {
    extra-substituters = [
      "https://nix-community.cachix.org"
    ];
    extra-trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };
}
