## ADDED Requirements

### Requirement: embedded manifest による解決

SchneeForge binary SHALL は build 時に `bootstrap-manifest.toml` を embed
する。`nix install` の manifest 解決は repo file を優先し、repo に file が
無い場合は embedded manifest を使う (fresh machine でも repo checkout が
不要)。

#### Scenario: repo に manifest がある場合はそちらを優先

- **WHEN** repo に `bootstrap-manifest.toml` が存在する
- **THEN** その内容を manifest として使う (現行挙動・dev / e2e 互換)

#### Scenario: repo が無い環境でも install できる

- **WHEN** repo checkout が存在しない (fresh machine)
- **THEN** embedded manifest で nix-installer の version / SHA256 pin を解決し、install を実行できる

#### Scenario: embed は build 時点の file に追従

- **WHEN** `bootstrap-manifest.toml` が更新される
- **THEN** 次回の build で embedded manifest が更新される (stale な embed で build された binary が存在しない)
