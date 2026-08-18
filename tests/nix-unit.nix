# machine input (defaults/machine.nix) の unit test。
# v2 P0: config.toml は廃止され、machine 情報は flake input として注入される。
# 実際の適用時は Rust 側が MachineFacts から生成した machine.nix で
# --override-input されるため、ここでは placeholder の構造を検証する。
let
  machine = import ../defaults/machine.nix;
in
{
  testMachineUsername = {
    expr = machine.username;
    expected = "schneeforge-user";
  };

  testMachineHome = {
    expr = machine.homeDirectory;
    expected = "/Users/schneeforge-user";
  };

  testMachineSystem = {
    expr = machine.system;
    expected = "aarch64-darwin";
  };

  testMachineHostname = {
    expr = machine.hostname;
    expected = "schneeforge-placeholder";
  };

  testMachineIsAttributeSet = {
    expr = builtins.isAttrs machine;
    expected = true;
  };
}
