{ pkgs ? import <nixpkgs> { } }:
with pkgs;
mkShell {
  buildInputs = [
    cargo
    rustc
    gcc
    rust-analyzer
    rustfmt
  ];

  RUST_SRC_PATH = "${rust.packages.stable.rustPlatform.rustLibSrc}";


}

