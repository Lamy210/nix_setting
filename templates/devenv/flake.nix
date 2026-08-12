{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    devenv.url = "github:cachix/devenv";
  };

  outputs =
    { nixpkgs, devenv, ... }@inputs:
    let
      inherit (nixpkgs) lib;
      forAllSystems = lib.genAttrs lib.systems.flakeExposed;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        devenv.lib.mkShell {
          inherit pkgs inputs;
          modules = [
            ({ pkgs, ... }: {
              packages = with pkgs; [
                go
                python3
                nodejs_24
              ];

              languages.rust.enable = true;

              services.postgres = {
                enable = true;
                initialScript = "CREATE DATABASE dev;";
              };

              services.redis.enable = true;

              scripts.build.exec = "go build ./...";
              scripts.test.exec = "go test ./...";

              enterShell = ''
                echo "dev ready"
              '';
            })
          ];
        }
      );
    };
}
