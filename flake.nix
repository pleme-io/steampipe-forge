{
  description = "steampipe-forge — Steampipe plugin table generator";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    substrate = {
      url = "github:pleme-io/substrate";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crate2nix = {
      url = "github:nix-community/crate2nix";
      flake = false;
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    devenv = {
      url = "github:cachix/devenv";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, substrate, crate2nix, fenix, devenv, ... }:
    let
      systems = [ "aarch64-darwin" "x86_64-linux" "aarch64-linux" ];

      forEachSystem = f: nixpkgs.lib.genAttrs systems (system:
        let
          rustLibrary = import "${substrate}/lib/rust-library.nix" {
            inherit system nixpkgs crate2nix devenv;
            nixLib = substrate;
          };
          result = rustLibrary {
            name = "steampipe-forge";
            src = ./.;
          };
        in f result
      );
    in {
      packages = forEachSystem (r: r.packages);
      devShells = forEachSystem (r: r.devShells);
      apps = forEachSystem (r: r.apps);
    };
}
