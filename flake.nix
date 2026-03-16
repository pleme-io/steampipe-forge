{
  description = "steampipe-forge — Steampipe plugin table generator library";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    substrate = {
      url = "github:pleme-io/substrate";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crate2nix = {
      url = "github:nix-community/crate2nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      substrate,
      crate2nix,
      ...
    }:
    let
      system = "aarch64-darwin";
      rustLibrary = import "${substrate}/lib/rust-library.nix" {
        inherit system nixpkgs;
        nixLib = substrate;
        inherit crate2nix;
      };
      lib = rustLibrary {
        name = "steampipe-forge";
        src = ./.;
      };
    in
    {
      packages.${system} = lib.packages;
      devShells.${system} = lib.devShells;
      apps.${system} = lib.apps;
      overlays.default = final: prev: {
        steampipe-forge = self.packages.${final.system}.default;
      };
      formatter.${system} = (import nixpkgs { inherit system; }).nixfmt-tree;
    };
}
