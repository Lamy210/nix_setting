{ pkgs, ... }:
{
  home.packages = with pkgs; [
    zsh
    (import ../git-patched.nix { inherit pkgs; })
    curl
    wget
    unzip

    starship
    fzf
    eza
    bat
    fd
    ripgrep
    zoxide
    atuin

    jq
    yq
    fx
    csvlens
    tealdeer
    sd
    broot
    xh
    procs
    tokei
    glow
    navi
    pandoc
    ouch
    bandwhich
    vim
    htop
    zsh-abbr
    zsh-completions
  ];
}
