_:
let
  userOptions = import ../../user-options/options.nix;
  hostOptions = import ./options.nix;
in
{
  imports = [ ../../modules/default.nix ];

  home.username = userOptions.username;
  home.homeDirectory = hostOptions.homeDirectory;
}
