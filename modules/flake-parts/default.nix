{ self, inputs, ... }:
{
  imports = [
    ./devshells.nix
    ./home.nix
    ./darwin.nix
  ];
}
