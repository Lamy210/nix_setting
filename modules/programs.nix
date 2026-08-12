{ ... }:
{
  programs.starship = {
    enable = true;
    settings = builtins.fromTOML (builtins.readFile ../config/starship/starship.toml);
  };

  programs.fzf = {
    enable = true;
    changeDirWidget.command = "";
    historyWidget.command = "";
  };

  programs.direnv = {
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

  programs.zoxide.enable = true;

  programs.atuin = {
    enable = true;
  };

  programs.bat = {
    enable = true;
    config = {
      theme = "Catppuccin Mocha";
      style = "plain,header,changes";
    };
  };

  programs.tmux = {
    enable = true;
    extraConfig = builtins.readFile ../config/tmux/tmux.conf;
  };
}
