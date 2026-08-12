_:
let
  userOptions = import ../../user-options/options.nix;
in
{
  imports = [
    ../../profiles/developer.nix
    ../../modules/default.nix
  ];

  home.username = userOptions.username;
  home.homeDirectory = "/Users/${userOptions.username}";
}
