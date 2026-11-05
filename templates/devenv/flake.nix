{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Candidate security-only package input; full lockfile and closure proof pending.
    node-security-nixpkgs.url = "github:NixOS/nixpkgs/a048ecb498d4f232d2963b19496bec9b196b990c";
    devenv.url = "github:cachix/devenv";
  };

  outputs =
    {
      nixpkgs,
      devenv,
      node-security-nixpkgs,
      ...
    }@inputs:
    let
      inherit (nixpkgs) lib;
      forAllSystems = lib.genAttrs lib.systems.flakeExposed;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          nodePatched = node-security-nixpkgs.legacyPackages.${system}.nodejs_24;
        in
        devenv.lib.mkShell {
          inherit pkgs inputs;
          modules = [
            ({ pkgs, ... }: {
              packages = with pkgs; [
                go
                python3
                nodePatched
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
