{ ... }:
{
  home.file.".gitconfig".source = ../config/git/gitconfig;

  xdg.configFile = {
    "wezterm/wezterm.lua".source = ../config/wezterm/wezterm.lua;
    "atuin/config.toml".source = ../config/atuin/config.toml;
    "mise/config.toml".source = ../config/mise/config.toml;
    "openspec/config.json".source = ../config/openspec/config.json;
    "lazygit/config.yml".source = ../config/lazygit/config.yml;
    "yazi/yazi.toml".source = ../config/yazi/yazi.toml;
    "broot/conf.toml".source = ../config/broot/conf.toml;
    "just/justfile".source = ../config/just/justfile;
  };
}
