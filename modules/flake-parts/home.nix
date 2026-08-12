{ inputs, ... }:
let
  userOptions = import ../../user-options/options.nix;
in
{
  flake = {
    homeConfigurations.default = inputs.home-manager.lib.homeManagerConfiguration {
      pkgs = import inputs.nixpkgs {
        system = "aarch64-darwin";
        config.allowUnfree = true;
      };
      modules = [
        ../../home.nix
        {
          home.username = userOptions.username;
          home.homeDirectory = userOptions.homeDirectory;
        }
      ];
    };
  };
}
