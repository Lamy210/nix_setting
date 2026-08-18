# Profile: minimal (v2 §17)。
# 端末の基礎のみ。developer (cli/git/dev/containers/db) に対して
# cli のみを import する。
_: {
  imports = [
    ../modules/packages/cli.nix
  ];
}
