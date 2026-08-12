{ ... }:
{
  perSystem =
    { pkgs, ... }:
    {
      packages.schneeforge = pkgs.rustPlatform.buildRustPackage {
        pname = "schneeforge";
        version = "0.1.0";
        src = ../..;
        cargoLock.lockFile = ../../Cargo.lock;
        meta = {
          description = "Declarative Developer Workstation Manager";
          mainProgram = "schneeforge";
        };
      };
    };
}
