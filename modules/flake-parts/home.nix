{ inputs, ... }:
let
  machine = import inputs.machine;
in
{
  flake = {
    homeConfigurations = {
      darwin-aarch64 = inputs.home-manager.lib.homeManagerConfiguration {
        pkgs = import inputs.nixpkgs {
          system = "aarch64-darwin";
          config.allowUnfree = true;
        };
        extraSpecialArgs = { inherit machine; };
        modules = [ ../../hosts/darwin-aarch64 ];
      };

      linux = inputs.home-manager.lib.homeManagerConfiguration {
        pkgs = import inputs.nixpkgs {
          system = "x86_64-linux";
          config.allowUnfree = true;
        };
        extraSpecialArgs = { inherit machine; };
        modules = [ ../../hosts/linux-generic ];
      };

      linux-arm = inputs.home-manager.lib.homeManagerConfiguration {
        pkgs = import inputs.nixpkgs {
          system = "aarch64-linux";
          config.allowUnfree = true;
        };
        extraSpecialArgs = { inherit machine; };
        modules = [ ../../hosts/linux-generic ];
      };
    };
  };
}

