{ inputs, ... }:
let
  userOptions = import ../../user-options/options.nix;
in
{
  flake = {
    homeConfigurations = {
      macbook-air = inputs.home-manager.lib.homeManagerConfiguration {
        pkgs = import inputs.nixpkgs {
          system = "aarch64-darwin";
          config.allowUnfree = true;
        };
        modules = [
          ../../hosts/macbook-air
          {
            home.username = userOptions.username;
            home.homeDirectory = userOptions.homeDirectory;
          }
        ];
      };

      linux = inputs.home-manager.lib.homeManagerConfiguration {
        pkgs = import inputs.nixpkgs {
          system = "x86_64-linux";
          config.allowUnfree = true;
        };
        modules = [
          ../../hosts/linux-generic
          {
            home.username = userOptions.username;
            home.homeDirectory = userOptions.homeDirectory;
          }
        ];
      };
    };
  };
}
