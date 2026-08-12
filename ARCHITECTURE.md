# ARCHITECTURE

nix_setting は Nix + Home Manager + nix-darwin によるクロスプラットフォーム開発環境基盤。

## 基本方針

| 役割 | 担当 | 例 |
|------|------|-----|
| Home Manager | ユーザーレベルのパッケージ + dotfiles | `home.packages`, `programs.*`, `xdg.configFile` |
| nix-darwin | macOS システム設定 + Homebrew | `system.defaults`, `homebrew.*` |
| devShell | プロジェクト固有の開発環境 | `devShells.*` |
| devenv | プロジェクトの service 起動 | `templates/devenv/` |

## ディレクトリ構成

```
nix_setting/
├── flake.nix                  # flake-parts エントリ + treefmt-nix
├── hosts/                     # ホスト固有設定
│   ├── macbook-air/           # macOS (homeDirectory=/Users/lamy210)
│   └── linux-generic/         # Linux (homeDirectory=/home/lamy210)
├── modules/
│   ├── flake-parts/           # devShells / homeConfigurations / darwinConfigurations
│   ├── packages/              # 用途別パッケージ (cli/git/dev/containers/db)
│   ├── experimental/          # opt-in モジュール (ai)
│   ├── shell.nix              # zsh + aliases + completions
│   ├── programs.nix           # starship/fzf/direnv/zoxide/atuin/bat/tmux
│   └── dotfiles.nix           # home.file + xdg.configFile
├── nix-darwin/                # macOS システム設定
│   ├── config/homebrew/       # Homebrew cask (Brewfile 代替)
│   ├── config/system.nix      # macOS defaults
│   └── config/nix-config.nix  # Nix GC + optimise
├── templates/                 # プロジェクトテンプレート (devenv/rust/node/python)
├── tests/                     # bats 統合テスト
└── .github/workflows/         # check / update / weekly
```

## Runtime 責務

詳細は `docs/runtime-ownership.md` を参照。

```
Global CLI       → Home Manager (Nix)
普段使い runtime → mise / FVM
Project runtime  → devShell / devenv
```

## 適用方法

```bash
# macOS (nix-darwin + home-manager 一括)
nh switch .#macbook-air

# Linux (home-manager のみ)
nh home switch .#linux
```

## 検証

```bash
nix flake check            # 評価 + treefmt check
nix build .#homeConfigurations.linux.activationPackage
nix build .#darwinConfigurations.macbook-air.system
```
