{
  inputs = {
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
    nixpkgs.url = "nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk.url = "github:nmattia/naersk";
  };
  outputs = { self, flake-utils, fenix, nixpkgs, naersk, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        # Add rust nightly to pkgs
        pkgs = nixpkgs.legacyPackages.${system} // { inherit (fenix.packages.${system}.latest) cargo rustc rust-src clippy-preview; inherit (fenix.packages.${system}) rust-analyzer; };

        naersk-lib = (naersk.lib."${system}".override {
          cargo = pkgs.cargo;
          rustc = pkgs.rustc;
        });

        dots = naersk-lib.buildPackage {
          pname = "dots";
          doCheck = true;
          cargoTestCommands = a: [
          ''USER=test-user cargo $cargo_options test $cargo_test_options''
          ''cargo clippy -- -D warnings''];
          nativeBuildInputs = with pkgs; [ pkg-config clippy-preview rustc];
          root = ./.;
        };


      in
      rec {
        packages.dots = dots;
        defaultPackage = dots;

        devShell = import ./shell.nix { inherit pkgs; };
      });
}
