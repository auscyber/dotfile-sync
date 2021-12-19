{ pkgs ? import <nixpkgs> {
    overlays = [
    (import "${fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz"}/overlay.nix")
      (self: super: {
          clippy-preview = super.fenix.latest.clippy-preview;
          rustc = super.fenix.latest.rustc;
          cargo  = super.fenix.latest.cargo;
          rust-src = super.fenix.latest.rust-src;
      }
        )
    ];
  } }:
with pkgs;
mkShell {
  buildInputs = [
    cargo
    rustc
    gcc
    rust-analyzer
    rustfmt
    lldb
    #        clippy
    cargo-edit
    clippy-preview
  ];

  RUST_SRC_PATH = "${pkgs.rust-src}/lib/rustlib/src/rust/library";

}

