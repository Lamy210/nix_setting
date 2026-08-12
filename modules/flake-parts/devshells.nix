{
  perSystem =
    { pkgs, ... }:
    let
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
          uv
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
          rustup
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
            go
            gotools
            golangci-lint
            gopls
            delve
            python3
            uv
            ruff
            pyright
            nodejs_24
            pnpm
            bun
            typescript
            typescript-language-server
            rustup
            cargo
            rustc
            rust-analyzer
            clippy
            git
            lazygit
            delta
            difftastic
            gh
            jq
            yq
            fx
            protobuf
            protoc-gen-go
            protoc-gen-go-grpc
            grpcurl
            docker
            docker-compose
            kubectl
            kubernetes-helm-wrapped
            pgcli
            usql
            redis
            sqlite
            terraform
            just
            hyperfine
            lefthook
            statix
            deadnix
            act
            nix-output-monitor
            actionlint
            shellcheck
            trufflehog
          ];
          shellHook = ''
            echo "dev shell ready"
            echo "go      $(go version 2>/dev/null || echo '-')"
            echo "python  $(python3 --version 2>/dev/null || echo '-')"
            echo "node    $(node --version 2>/dev/null || echo '-')"
            echo "rustc   $(rustc --version 2>/dev/null || echo '-')"
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
