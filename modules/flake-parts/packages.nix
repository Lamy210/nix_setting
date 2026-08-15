_: {
  perSystem =
    { pkgs, ... }:
    let
      linuxBuildInputs = with pkgs; [
        webkitgtk_4_1
        gtk3
        libayatana-appindicator
        librsvg
      ];
    in
    {
      packages.schneeforge = pkgs.rustPlatform.buildRustPackage {
        pname = "schneeforge";
        version = "0.2.0-rc.5";
        src = ../..;
        cargoLock.lockFile = ../../Cargo.lock;
        meta = {
          description = "Declarative Developer Workstation Manager";
          mainProgram = "schneeforge";
        };
      };

      packages.schneeforge-desktop = pkgs.rustPlatform.buildRustPackage {
        pname = "schneeforge-desktop";
        version = "0.2.0-rc.5";
        src = ../..;
        cargoHash = "sha256-7+VTcjt7+N0NRW0/dSj01VqYSksCXDi89wMPVo8pyn4=";
        nativeBuildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [
          pkgs.pkg-config
        ];
        buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux linuxBuildInputs;
        cargoBuildCommand = "cargo build --release --manifest-path apps/desktop/src-tauri/Cargo.toml";
        cargoCheckCommand = "cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml";
        cargoTestCommand = null;
        meta = {
          description = "SchneeForge Desktop (Tauri 2)";
          mainProgram = "schneeforge-desktop";
        };
      };
    };
}
