{
  description = "Portable terminal environment managed by Nix + Home Manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # Candidate: isolate the Node 24 security backport from the main Nixpkgs input.
    # Do not accept for release without committed input lock and runtime closure proof.
    node-security-nixpkgs.url = "github:NixOS/nixpkgs/a048ecb498d4f232d2963b19496bec9b196b990c";

    # machine 固有情報 (username / homeDirectory / system / hostname)。
    # repo には placeholder を同梱し、apply 時に SchneeForge が生成した
    # machine.nix へ --override-input で差し替える (path input のため
    # lock への commit は不要)
    machine = {
      url = "path:./defaults/machine.nix";
      flake = false;
    };

    # 選択 profile (manifest [profiles] のいずれか)。placeholder は
    # null で、apply 時に SchneeForge が生成した profile input へ
    # --override-input で差し替える
    profile = {
      url = "path:./defaults/profile.nix";
      flake = false;
    };

    flake-parts.url = "github:hercules-ci/flake-parts";

    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    nix-darwin = {
      url = "github:LnL7/nix-darwin";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ ./modules/flake-parts ];
      systems = [
        "aarch64-darwin"
        "x86_64-linux"
        "aarch64-linux"
      ];
      perSystem = { system, ... }: {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };
      };
    };
}
