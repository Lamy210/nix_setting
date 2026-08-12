{ inputs, ... }:
{
  imports = [
    inputs.treefmt-nix.flakeModule
    ./devshells.nix
    ./home.nix
    ./darwin.nix
  ];

  perSystem = { ... }: {
    treefmt.config = {
      projectRootFile = "flake.nix";
      programs = {
        nixfmt.enable = true;
        shfmt.enable = true;
        taplo.enable = true;
      };
      settings = {
        global.excludes = [
          "flake.lock"
          ".gitignore"
          ".envrc"
          "*.jpg"
          "*.png"
          "*.gif"
          "*.webp"
        ];
        shfmt.includes = [ "*.sh" ];
      };
    };
  };
}
