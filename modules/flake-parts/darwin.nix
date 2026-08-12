{ inputs, ... }:
let
  userOptions = import inputs.user-options;
in
{
  flake = {
    darwinConfigurations.default = inputs.nix-darwin.lib.darwinSystem {
      system = "aarch64-darwin";
      modules = [
        ../../nix-darwin/default.nix
        inputs.home-manager.darwinModules.home-manager
        {
          home-manager = {
            useGlobalPkgs = true;
            useUserPackages = true;
            users.${userOptions.username} = {
              imports = [ ../../home.nix ];
              home.homeDirectory = userOptions.homeDirectory;
            };
          };
        }
      ];
    };
  };
}
