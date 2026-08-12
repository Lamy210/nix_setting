{ pkgs, ... }:
let
  homebrew = import ./config/homebrew/default.nix { inherit pkgs; };
  nixConfig = import ./config/nix-config.nix { inherit pkgs; };
  systemConfig = import ./config/system.nix { inherit pkgs; };
in
{
  imports = [
    homebrew
    nixConfig
    systemConfig
  ];
  system.stateVersion = 5;
}
