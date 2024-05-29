with import <nixpkgs> {};
mkShell {
  NIX_LD_LIBRARY_PATH = lib.makeLibraryPath [
    stdenv.cc.cc
    openssl
    pkg-config
    graphite2
    icu
    freetype
    fontconfig
    harfbuzz   
    libpng
  ];
  LD_LIBRARY_PATH = lib.makeLibraryPath [
    stdenv.cc.cc
    openssl
    pkg-config
    graphite2
    icu
    freetype
    fontconfig
    harfbuzz
    libpng
  ];
  NIX_LD = lib.fileContents "${stdenv.cc}/nix-support/dynamic-linker";
  buildInputs = [
    stdenv.cc.cc
    openssl
    pkg-config
    graphite2
    icu
    freetype
    fontconfig
    harfbuzz
    libpng
  ];
}
