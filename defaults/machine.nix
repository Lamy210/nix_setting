# machine input の placeholder。
# clone 直後の `nix flake check` 等が通るように同梱している。
# SchneeForge の apply / plan は実行環境から検出した MachineFacts で
# 生成した machine.nix へ --override-input machine <path> で差し替える
# ため、この値が実際の適用に使われることはない。
{
  username = "schneeforge-user";
  homeDirectory = "/Users/schneeforge-user";
  system = "aarch64-darwin";
  hostname = "schneeforge-placeholder";
}
