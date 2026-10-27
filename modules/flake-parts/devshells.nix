{
  perSystem =
    { pkgs, ... }:
    let
      # Security-only package backport (emSign/eMudhra): vendor 0.13.5.
      uvPatched = pkgs.uv.overrideAttrs (final: previous: {
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
      });
      devShellPackages = {
        go = with pkgs; [
          go
          gotools
          golangci-lint
          gopls
          delve
          protobuf
          protoc-gen-go
          protoc-gen-go-grpc
        ];
        python = with pkgs; [
          python3
          uvPatched
          ruff
          pyright
        ];
        node = with pkgs; [
          nodejs_24
          pnpm
          bun
          typescript
          typescript-language-server
        ];
        rust = with pkgs; [
          cargo
          rustc
          rust-analyzer
          clippy
        ];
      };
    in
    {
      devShells = {
        default = pkgs.mkShell {
          packages = with pkgs; [
            git
            lazygit
            delta
            difftastic
            gh
            jq
            yq
            fx
            just
            hyperfine
            lefthook
            statix
            deadnix
            actionlint
            shellcheck
            trufflehog
            act
            nix-output-monitor
            nixd
            omnix
            devenv
            nix-tree
            nix-melt
            nix-unit
            cargo
            rustc
            rustfmt
            clippy
            cargo-tauri
          ];
          shellHook = ''
            echo "repo maintenance shell ready"
            echo "lint     statix + deadnix + actionlint + shellcheck"
            echo "format   nix fmt (nixfmt + shfmt + taplo)"
            echo "lang     nix develop .#go / .#python / .#node / .#rust"
          '';
        };
        go = pkgs.mkShell {
          packages = devShellPackages.go;
          shellHook = ''
            echo "go    $(go version 2>/dev/null || echo 'not found')"
            echo "gopls $(gopls version 2>/dev/null || echo 'not found')"
          '';
        };
        python = pkgs.mkShell {
          packages = devShellPackages.python;
          shellHook = ''
            echo "python  $(python3 --version 2>/dev/null || echo 'not found')"
            echo "uv      $(uv --version 2>/dev/null || echo 'not found')"
          '';
        };
        node = pkgs.mkShell {
          packages = devShellPackages.node;
          shellHook = ''
            echo "node  $(node --version 2>/dev/null || echo 'not found')"
            echo "pnpm  $(pnpm --version 2>/dev/null || echo 'not found')"
            echo "bun   $(bun --version 2>/dev/null || echo 'not found')"
          '';
        };
        rust = pkgs.mkShell {
          packages = devShellPackages.rust;
          shellHook = ''
            echo "rustc $(rustc --version 2>/dev/null || echo 'not found')"
            echo "cargo $(cargo --version 2>/dev/null || echo 'not found')"
          '';
        };
        k8s = pkgs.mkShell {
          packages = with pkgs; [
            kubectl
            kubernetes-helm-wrapped
            kind
            docker
            docker-compose
            colima
            k9s
          ];
          shellHook = ''
            echo "kubectl $(kubectl version --client --short 2>/dev/null || echo '-')"
            echo "helm    $(helm version --short 2>/dev/null || echo '-')"
          '';
        };
        db = pkgs.mkShell {
          packages = with pkgs; [
            pgcli
            mycli
            usql
            redis
            sqlite
            protobuf
          ];
          shellHook = ''
            echo "pgcli  $(pgcli --version 2>/dev/null || echo '-')"
            echo "mycli  $(mycli --version 2>/dev/null || echo '-')"
            echo "usql   $(usql --version 2>/dev/null || echo '-')"
          '';
        };
      };
    };
}
