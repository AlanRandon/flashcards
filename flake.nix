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

  outputs =
    {
      nixpkgs,
      fenix,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (system: {
      devShells.default =
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [
              (
                _: super:
                let
                  pkgs = fenix.inputs.nixpkgs.legacyPackages.${super.system};
                in
                fenix.overlays.default pkgs pkgs
              )
            ];
          };
          libraries = with pkgs; [
            stdenv.cc
            openssl
            graphite2
            icu72
            freetype
            fontconfig
            harfbuzz
            libpng
            zlib
          ];
          toolchain = fenix.packages."${system}".complete.toolchain;
        in
        pkgs.mkShell {
          nativeBuildInputs =
            (with pkgs; [
              pkg-config
              mold
            ])
            ++ [
              toolchain
            ];

          buildInputs =
            (with pkgs; [
              poppler-utils
            ])
            ++ libraries;

          packages = [
            pkgs.rust-analyzer-nightly
            pkgs.sqlx-cli
            pkgs.sqlite
            pkgs.tailwindcss_4
            pkgs.esbuild
            pkgs.nodejs
            pkgs.djlint
          ];

          DATABASE_URL = "sqlite:dev.db";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libraries;
        };
    });

  nixConfig = {
    extra-substituters = [
      "https://nix-community.cachix.org" # "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
    ];
  };
}
