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
            users.users.${userOptions.username}.home = userOptions.homeDirectory;
            home-manager = {
              useGlobalPkgs = true;
              useUserPackages = true;
              users.${userOptions.username} = import ../../hosts/macbook-air;
            };
          }
        ];
      };
    };
  };
}
