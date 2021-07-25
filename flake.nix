
{
  inputs = {
    flake-compat =  {
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
  outputs = { self, flake-utils, fenix, nixpkgs, naersk,  ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        # Add rust nightly to pkgs
        pkgs = nixpkgs.legacyPackages.${system} // { inherit (fenix.packages.${system}.default) cargo rustc rust-src clippy-preview;  };

        naersk-lib = (naersk.lib."${system}".override {
          cargo = pkgs.cargo;
          rustc = pkgs.rustc;
        });

        dots = naersk-lib.buildPackage {
          pname = "dots";
          nativeBuildInputs = with pkgs; [pkg-config];
          root = ./.;
        };


      in
      rec {
        packages.dots = dots;
        defaultPackage = dots;

        devShell = import ./shell.nix { inherit pkgs; };
      });
}
