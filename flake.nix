{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        kicad-wrapper = pkgs.callPackage ./derivation.nix { };
      in
      {
        checks = {
          inherit kicad-wrapper;
        };
        packages = {
          inherit kicad-wrapper;
          default = kicad-wrapper;
        };
      }
    );
}
