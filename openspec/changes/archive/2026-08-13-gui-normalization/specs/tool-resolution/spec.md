## ADDED Requirements

### Requirement: PATH 非依存のツール解決
ツール解決 SHALL は PATH だけでなく既知パスも探索する。macOS GUI は Terminal と異なる PATH を持つため。

#### Scenario: PATH に無いが既知パスにある
- **WHEN** ツールが PATH に無いが `/nix/var/nix/profiles/default/bin` に存在する
- **THEN** 解決結果は available: true とその path を返す

#### Scenario: 解決順序
- **WHEN** ツールが複数箇所に存在する
- **THEN** PATH → /nix/var/nix/profiles/default/bin → ~/.nix-profile/bin → /opt/homebrew/bin → /usr/local/bin の順で解決する

### Requirement: 実行時の解決済みパス利用
コマンド実行 SHALL は解決済みの絶対パスを使う。

#### Scenario: nh が未解決でも nix-darwin 適用できる
- **WHEN** `nh` が未インストールの fresh machine で apply する
- **THEN** core は `nh` に依存せず `nix` の解決済みパスで適用する
