{ pkgs, ... }:
{
  home.packages = with pkgs; [
    gh
    ghq
    lazygit
    delta
    difftastic
  ];
}
