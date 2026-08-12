{ ... }:
{
  imports = [
    ./packages.nix
    ./shell.nix
    ./programs.nix
    ./dotfiles.nix
  ];

  home.stateVersion = "24.11";
  programs.home-manager.enable = true;
}
