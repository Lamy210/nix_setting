{ inputs, ... }:
let
  userOptions = import ../../user-options/options.nix;
in
{
  flake = {
    darwinConfigurations = {
      macbook-air = inputs.nix-darwin.lib.darwinSystem {
        system = "aarch64-darwin";
        modules = [
          ../../nix-darwin/default.nix
          inputs.home-manager.darwinModules.home-manager
          {
            home-manager = {
              useGlobalPkgs = true;
              useUserPackages = true;
              users.${userOptions.username} = {
                imports = [ ../../hosts/macbook-air ];
                home.homeDirectory = userOptions.homeDirectory;
              };
            };
          }
        ];
      };
    };
  };
}
