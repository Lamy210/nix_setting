{ machine, ... }:
{
  imports = [
    ../../profiles/developer.nix
    ../../modules/default.nix
  ];

  home.username = machine.username;
  home.homeDirectory = machine.homeDirectory;
}
