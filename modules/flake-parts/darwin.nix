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
            system.primaryUser = userOptions.username;
            users.users.${userOptions.username}.home = "/Users/${userOptions.username}";
            nixpkgs.config.allowUnfree = true;
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
