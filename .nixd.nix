{
  formatting.command = [
    "nix"
    "fmt"
  ];
  options = {
    nixos.expr = "(builtins.getFlake (toString ./.)).nixosConfigurations or {}";
    home-manager.expr = "(builtins.getFlake (toString ./.)).homeConfigurations or {}";
    nix-darwin.expr = "(builtins.getFlake (toString ./.)).darwinConfigurations or {}";
  };
}
