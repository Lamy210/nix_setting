{ ... }:
let
  userOptions = import ../../user-options/options.nix;
in
{
  imports = [ ../../modules/default.nix ];

  home.username = userOptions.username;
  home.homeDirectory = "/home/lamy210";
}
