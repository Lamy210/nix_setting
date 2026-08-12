_: {
  programs = {
    starship = {
      enable = true;
      settings = builtins.fromTOML (builtins.readFile ../config/starship/starship.toml);
    };

    fzf = {
      enable = true;
      changeDirWidget.command = "";
      historyWidget.command = "";
    };

    direnv = {
      enable = true;
      nix-direnv.enable = true;
      config = {
        global = {
          load_dotenv = false;
          strict_env = true;
          warn_timeout = "5s";
        };
      };
    };

    zoxide.enable = true;

    atuin.enable = true;

    bat = {
      enable = true;
      config = {
        theme = "Catppuccin Mocha";
        style = "plain,header,changes";
      };
    };

    tmux = {
      enable = true;
      extraConfig = builtins.readFile ../config/tmux/tmux.conf;
    };
  };
}
