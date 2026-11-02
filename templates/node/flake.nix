{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        # Security backport: nixpkgs locked on 2026-10-27 still ships Bun 1.4.2.
        # Bun 1.4.4 fixes CVE-2026-111497 (bundled Brotli OOB write).
        # Source hashes match the vendor's GitHub release asset SHA256 digests.
        # Remove this override only after the pinned nixpkgs supplies >=1.4.4.
        bunPatched = pkgs.bun.overrideAttrs (
          final: previous: {
            version = "1.4.4";
            src = final.passthru.sources.${pkgs.stdenv.hostPlatform.system};
            passthru = previous.passthru // {
              sources = {
                "aarch64-darwin" = pkgs.fetchurl {
                  url = "https://github.com/oven-sh/bun/releases/download/bun-v1.4.4/bun-darwin-aarch64.zip";
                  hash = "sha256-wkDX8My5DCY6hJ/NsJt+5iYw2SE0jtaq/ahrnVd7xRA=";
                };
                "aarch64-linux" = pkgs.fetchurl {
                  url = "https://github.com/oven-sh/bun/releases/download/bun-v1.4.4/bun-linux-aarch64.zip";
                  hash = "sha256-uqa7NvUdblhWHgK76oJQOKnfRU9nPW13bou/hvjHhQ4=";
                };
                "x86_64-linux" = pkgs.fetchurl {
                  url = "https://github.com/oven-sh/bun/releases/download/bun-v1.4.4/bun-linux-x64-baseline.zip";
                  hash = "sha256-4ERutVmVdy65iUvrrppr42jZhai5Jx7hJEhvmUbu3XY=";
                };
              };
            };
          }
        );
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            nodejs_24
            pnpm
            bunPatched
            typescript
            typescript-language-server
          ];
          shellHook = ''
            echo "node $(node --version) | pnpm $(pnpm --version) | bun $(bun --version)"
          '';
        };
      }
    );
}
