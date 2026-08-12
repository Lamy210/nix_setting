_:
let
  manifest = builtins.fromTOML (builtins.readFile ../../config.toml);
in
{
  imports = [
    ../../profiles/developer.nix
    ../../modules/default.nix
  ];

  home.username = manifest.user.username;
  home.homeDirectory = "/Users/${manifest.user.username}";
}
