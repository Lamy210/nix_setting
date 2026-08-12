# Copy this to your project root as a starting point.
# Run: nix develop
# Or:  devenv up (starts postgres + redis)

{ pkgs, ... }:

{
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
    echo "dev ready | go $(go version) | rustc $(rustc --version)"
  '';
}
