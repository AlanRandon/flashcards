{
  description = "Nix flake for flashcards";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
  };

  outputs = { nixpkgs, ... }:
    let
      system = "x86_64-linux";
    in
    {
      devShells."${system}".default =
        let
          inherit (pkgs) lib;
          pkgs = import nixpkgs {
            inherit system;
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
        in
        pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = libraries;
          LD_LIBRARY_PATH = lib.makeLibraryPath libraries;
          shellHook = "exec \${SHELL:=sh}";
        };
    };
}
