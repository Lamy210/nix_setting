{ inputs, ... }:
let
  machine = import inputs.machine;
in
{
  flake = {
    darwinConfigurations = {
      darwin-aarch64 = inputs.nix-darwin.lib.darwinSystem {
        system = "aarch64-darwin";
        modules = [
          ../../nix-darwin/default.nix
          inputs.home-manager.darwinModules.home-manager
          {
            system.primaryUser = machine.username;
            users.users.${machine.username}.home = machine.homeDirectory;
            nixpkgs.config.allowUnfree = true;
            home-manager = {
              useGlobalPkgs = true;
              useUserPackages = true;
              extraSpecialArgs = { inherit machine; };
              users.${machine.username} = import ../../hosts/darwin-aarch64;
            };
          }
        ];
      };
    };
  };
}
