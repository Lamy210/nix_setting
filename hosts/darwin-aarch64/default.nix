{
  machine,
  profileModule,
  ...
}:
{
  imports = [
    profileModule
    ../../modules/default.nix
  ];

  home.username = machine.username;
  home.homeDirectory = machine.homeDirectory;
}
