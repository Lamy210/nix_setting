{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    devenv.url = "github:cachix/devenv";
  };

  outputs =
    inputs@{ nixpkgs, devenv, ... }:
    devenv.lib.mkFlake {
      inherit inputs;
      modules = [ ./devenv.nix ];
    };
}
