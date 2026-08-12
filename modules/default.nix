_: {
  imports = [
    ./shell.nix
    ./programs.nix
    ./dotfiles.nix
    # ./experimental/ai.nix  # opt-in: claude-code + gemini-cli + github-copilot-cli
  ];

  home.stateVersion = "24.11";
  programs.home-manager.enable = true;
}
