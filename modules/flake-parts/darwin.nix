{ inputs, ... }:
let
  manifest = builtins.fromTOML (builtins.readFile ../../config.toml);
  username = manifest.user.username;
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
            system.primaryUser = username;
            users.users.${username}.home = "/Users/${username}";
            nixpkgs.config.allowUnfree = true;
            home-manager = {
              useGlobalPkgs = true;
              useUserPackages = true;
              users.${username} = import ../../hosts/macbook-air;
            };
          }
        ];
      };
    };
  };
}
