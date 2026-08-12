{ ... }:
let
  homebrew = import ./config/homebrew/default.nix;
  nixConfig = import ./config/nix-config.nix;
  systemConfig = import ./config/system.nix;
in
{
  imports = [
    homebrew
    nixConfig
    systemConfig
  ];
  system.stateVersion = 5;
}
