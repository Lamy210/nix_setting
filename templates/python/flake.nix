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
        # Security-only package backport (emSign/eMudhra): vendor 0.13.5.
        uvPatched = pkgs.uv.overrideAttrs (
          final: previous: {
          version = "0.13.5";
          src = pkgs.fetchFromGitHub {
            owner = "astral-sh";
            repo = "uv";
            tag = "0.13.5";
            hash = "sha256-SXTusm2EJ49tk03c3vSHcZp3AtAuj9SGOzsMwpnjXDw=";
          };
          cargoHash = "sha256-spQmSFAjNs6pWnw2nd/d3ZysIMK9j+BjDjrwcnUCmWw=";
          # Override the precomputed vendored Rust dependencies, not just cargoHash.
          cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
            inherit (final) src version;
            pname = previous.pname;
            hash = final.cargoHash;
          };
          }
        );
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            python3
            uvPatched
            ruff
            pyright
          ];
          shellHook = ''
            echo "python $(python3 --version) | uv $(uv --version)"
          '';
        };
      }
    );
}
