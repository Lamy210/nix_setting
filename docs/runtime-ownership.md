# Runtime Management Ownership

## 責務分離

| 層 | 管理 | 対象 | 例 |
|----|------|------|-----|
| OS / global CLI | Home Manager (Nix) | 常時使うCLIツール | git, zsh, eza, bat, docker |
| 普段使い runtime | mise / FVM | バージョン切り替えが必要な言語 | node (mise), Flutter (FVM) |
| project reproducibility | nix devShell / devenv | プロジェクト固有の固定バージョン | go, python3, nodejs_24, rustup |

## 各ツールの責務

| ツール | 責務 | 場所 |
|--------|------|------|
| **Nix / Home Manager** | OS レベルの CLI ツール + dotfiles | `home.packages`, `programs.*` |
| **nix devShell** | プロジェクト固有の開発環境（固定バージョン） | `devShells.*` |
| **mise** | グローバルな言語ランタイムのバージョン管理 | `config/mise/config.toml` |
| **FVM** | Flutter/Dart のバージョン管理 | `$HOME/fvm` |
| **rustup** | Rust toolchain 管理（グローバル） | `home.packages` |
| **bun** | Bun ランタイム（`$HOME/.bun`） | shell.nix の PATH 設定 |

## Rust のルール

- **グローバル**: `rustup` で管理（`rustup default stable`）
- **devShell**: `rustup` + `rustc` + `cargo` + `clippy` + `rust-analyzer` を Nix から提供
- プロジェクトで特定バージョンが必要なら `rust-toolchain.toml` + `rustup` で上書き

## 避けるべきこと

- devShell と Home Manager で同じ言語ランタイムの重複インストール
- `command -v node` の結果が環境によって変わる（PATH 競合）
- mise と Nix が同じバージョンを別々に管理
