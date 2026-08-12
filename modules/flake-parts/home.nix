{ inputs, ... }:
{
  flake = {
    homeConfigurations = {
      macbook-air = inputs.home-manager.lib.homeManagerConfiguration {
        pkgs = import inputs.nixpkgs {
          system = "aarch64-darwin";
          config.allowUnfree = true;
        };
        modules = [ ../../hosts/macbook-air ];
      };

      linux = inputs.home-manager.lib.homeManagerConfiguration {
        pkgs = import inputs.nixpkgs {
          system = "x86_64-linux";
          config.allowUnfree = true;
        };
        modules = [ ../../hosts/linux-generic ];
      };

      linux-arm = inputs.home-manager.lib.homeManagerConfiguration {
        pkgs = import inputs.nixpkgs {
          system = "aarch64-linux";
          config.allowUnfree = true;
        };
        modules = [ ../../hosts/linux-generic ];
      };
    };
  };
}
