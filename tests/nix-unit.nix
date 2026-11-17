# machine / profile input の既存動作の unit test。
# v2 P0: config.toml は廃止され、machine 情報は flake input として注入される。
# 実際の適用時は Rust 側が MachineFacts から生成した machine.nix で
# --override-input されるため、ここでは placeholder の構造を検証する。
let
  machine = import ../defaults/machine.nix;
  resolveProfile =
    profile:
    ((import ../modules/profile-input.nix) { inputs = { inherit profile; }; }).flake.profileModules;
  defaultProfile = resolveProfile ../defaults/profile.nix;
  minimalProfile = resolveProfile ./fixtures/profile-minimal.nix;
  emptyProfile = resolveProfile ./fixtures/profile-empty.nix;
  unknownProfile = resolveProfile ./fixtures/profile-unknown.nix;
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

  # profileModules.name のみでは unknown profile の throw が遅延評価で
  # 検出されないため、module フィールドを明示的に評価する。
  testDefaultProfileName = {
    expr = defaultProfile.name;
    expected = "developer";
  };

  testDefaultProfileModule = {
    expr = defaultProfile.module { };
    expected = (import ../profiles/developer.nix) { };
  };

  testMinimalProfileName = {
    expr = minimalProfile.name;
    expected = "minimal";
  };

  testMinimalProfileModule = {
    expr = minimalProfile.module { };
    expected = (import ../profiles/minimal.nix) { };
  };

  testEmptyProfileFallsBackToDeveloper = {
    expr = {
      inherit (emptyProfile) name;
      module = emptyProfile.module { };
    };
    expected = {
      name = "developer";
      module = (import ../profiles/developer.nix) { };
    };
  };

  testUnknownProfileDoesNotFallBack = {
    expr = builtins.tryEval unknownProfile.module;
    expected = {
      success = false;
      value = false;
    };
  };
}
