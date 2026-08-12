{ pkgs, ... }:
{
  home.packages = with pkgs; [
    yazi
    btop
    dust
    duf
    gdu
    just
    direnv
    mise
    tmux
    hyperfine
    rustup
    watchman
    ruby
    python3
    nh
    nix-index
  ];
}
