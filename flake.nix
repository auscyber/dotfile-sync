{
  inputs = {
    nixpkgs.follows = "unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = {self, nixpkgs, flake-utils, ...}: 
  flake-utils.lib.eachDefaultSystem (system:{
    devShell = import ./shell.nix { inherit pkgs;};
  });
}
