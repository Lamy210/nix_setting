# profile input の解決 helper。
# 選択された profile 名から profiles/<name>.nix を import する。
# placeholder (null) の場合は manifest の default と同じ developer に
# fallback するため、clone 直後の flake 評価も現行と同じ構成になる。
{
  inputs,
  ...
}:
let
  selected = import inputs.profile;
  # null (未選択) は manifest の [profiles] default と同一の developer へ。
  # ここに manifest を parse させず hard-code しているのは、flake input
  # の段階で toml 依存を持ちたくないため。core 側が manifest default を
  # 注入するため、実際の適用でこの値が使われるのは未選択時のみ。
  name = if isNull selected.profile || selected.profile == "" then "developer" else selected.profile;
  file = ../profiles + "/${name}.nix";
in
{
  flake = {
    profileModules = {
      inherit name;
      module = if builtins.pathExists file then import file else throw "unknown profile: ${name}";
    };
  };
}
