{
  description = "Portable terminal environment managed by Nix + Home Manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-darwin = {
      url = "github:LnL7/nix-darwin";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, home-manager, nix-darwin, ... }:
  let
    userOptions =
      let f = ./user-options.nix;
      in if builtins.pathExists f then import f
      else {
        username = "runner";
        homeDirectory = "/home/runner";
        system = "x86_64-linux";
      };

    pkgs = import nixpkgs {
      system = userOptions.system;
      config.allowUnfree = true;
    };

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
  in {
    formatter.${userOptions.system} = pkgs.nixfmt-rfc-style;

    homeConfigurations.default = home-manager.lib.homeManagerConfiguration {
      inherit pkgs;

      modules = [
        ./home.nix
        {
          home.username = userOptions.username;
          home.homeDirectory = userOptions.homeDirectory;
        }
      ];
    };

    devShells.${userOptions.system} = {
      default = pkgs.mkShell {
        packages = with pkgs; [
          go gotools golangci-lint gopls delve
          python3 uv ruff pyright
          nodejs_24 pnpm bun typescript typescript-language-server
          rustup cargo rustc rust-analyzer clippy
          git lazygit delta difftastic gh
          jq yq fx
          protobuf protoc-gen-go protoc-gen-go-grpc grpcurl
          docker docker-compose kubectl helm
          pgcli usql redis sqlite
          terraform
          just watchexec hyperfine
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
        '';
      };
      k8s = pkgs.mkShell {
        packages = with pkgs; [
          kubectl helm kind
          docker docker-compose colima
          k9s
        ];
        shellHook = ''
          echo "kubectl $(kubectl version --client --short 2>/dev/null || echo '-')"
          echo "helm    $(helm version --short 2>/dev/null || echo '-')"
        '';
      };
      db = pkgs.mkShell {
        packages = with pkgs; [
          pgcli mycli usql
          redis sqlite
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
