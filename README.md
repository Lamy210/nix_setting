# nix_setting

<img src="assets/icon.svg" width="96" height="96" alt="SchneeForge logo">

Nix + Home Manager + nix-darwin によるクロスプラットフォーム開発環境基盤。

- **対象**: Apple Silicon Mac / Linux x86_64 / Linux aarch64
- **管理**: パッケージ・dotfiles・macOS設定・Homebrew cask を宣言的に管理
- **検証**: CI で評価・ビルド・lint・secret scan を自動実行

## 構成

詳細は [ARCHITECTURE.md](./ARCHITECTURE.md) を参照。

```
nix_setting/
├── hosts/         # ホスト固有設定 (macbook-air / linux-generic)
├── modules/       # Home Manager モジュール (packages / shell / programs / dotfiles)
├── nix-darwin/    # macOS システム + Homebrew 設定
├── templates/     # プロジェクトテンプレート (devenv / rust / node / python / flutter)
├── tests/         # bats + nix-unit テスト
└── .github/       # CI (check / update / weekly)
```

## 導入手順

### ワンライナー (推奨)

```bash
curl -fsSL https://raw.githubusercontent.com/Lamy210/nix_setting/main/install.sh | bash
```

### 手動

```bash
# 1. Nix インストール
curl -L https://nixos.org/nix/install | sh

# 2. クローン & 適用 (OS/arch を自動検出)
git clone https://github.com/Lamy210/nix_setting.git "$HOME/nix_setting"
cd "$HOME/nix_setting"
./bootstrap.sh
```

### 診断のみ

```bash
nix run github:Lamy210/nix_setting#doctor
```

### CLI バイナリ (Nix 不要)

GitHub Release から対応プラットフォームのバイナリを取得:

```bash
# v0.1.0 以降の Release から schneeforge-<os>-<arch> をダウンロード
chmod +x schneeforge-aarch64-darwin
./schneeforge-aarch64-darwin doctor
```

### Rust (cargo install)

```bash
cargo install --git https://github.com/Lamy210/nix_setting schneeforge
```

### デスクトップ GUI (Tauri)

```bash
nix build github:Lamy210/nix_setting#schneeforge-desktop
# または開発用
cd apps/desktop/src-tauri && cargo tauri dev
```

## プロジェクトテンプレート

```bash
nix flake init -t github:Lamy210/nix_setting#rust
nix flake init -t github:Lamy210/nix_setting#node
nix flake init -t github:Lamy210/nix_setting#python
nix flake init -t github:Lamy210/nix_setting#flutter
nix flake init -t github:Lamy210/nix_setting#devenv
```

## 日常利用

```bash
# 設定変更を適用
nh darwin switch .#darwinConfigurations.macbook-air   # macOS (nix-darwin + home-manager)
nh home switch .#homeConfigurations.linux             # Linux (home-manager)

# フォーマット
nix fmt

# 検証
nix flake check
```

## devShell

```bash
nix develop            # nix_setting メンテナンス用 (lint/format/nix診断)
nix develop .#go       # Go
nix develop .#python   # Python
nix develop .#node     # Node.js
nix develop .#rust     # Rust
nix develop .#k8s      # Kubernetes
nix develop .#db       # DB
```

## プロジェクトテンプレート

```bash
# テンプレートを新規プロジェクトにコピー
cp -r ~/nix_setting/templates/rust ./my-rust-project
```

| テンプレート | 内容 |
|-------------|------|
| `devenv/` | Go + Python + Node + Rust + PostgreSQL + Redis |
| `rust/` | cargo + rustc + clippy + rust-analyzer |
| `node/` | Node.js 24 + pnpm + bun + TypeScript |
| `python/` | Python 3 + uv + ruff + pyright |
| `flutter/` | Flutter + Dart + JDK |

## Homebrew casks

Brewfile は廃止。nix-darwin で宣言管理。

```bash
nh darwin switch .#darwinConfigurations.macbook-air   # Homebrew cask も自動適用
```

## CI

| ジョブ | 内容 |
|--------|------|
| flake-check | nix flake check + Linux activation build |
| macos-check | macOS の flake check + HM/nix-darwin build |
| docker-check | Docker サンドボックス検証 |
| lint | actionlint + shellcheck + statix + deadnix |
| secret-scan | trufflehog + 画像チェック |
| devshell-smoke | devShell 起動 + runtime 確認 |
| template-check | テンプレート flake 検証 |
| bootstrap-test | bats 統合テスト |

## ドキュメント

- [ARCHITECTURE.md](./ARCHITECTURE.md) — 設計方針・責務分離
- [docs/runtime-ownership.md](./docs/runtime-ownership.md) — runtime 管理の責務
- [docs/terminal-spec.md](./docs/terminal-spec.md) — 全体仕様

## カスタマイズ

### ユーザー名 (manifest)

`config.toml` が唯一のユーザー情報源。homeDirectory は username から自動派生。

```toml
# config.toml
schema = 1

[user]
username = "yourname"
```

スキーマ検証は `config.schema.json`。

### パッケージ追加

パッケージは `modules/packages/*.nix` に追記。プロファイル集約は `profiles/developer.nix`。

```nix
# modules/packages/cli.nix
home.packages = with pkgs; [
  ripgrep  # 追加
];
```

```bash
nh darwin switch .#darwinConfigurations.macbook-air  # 再適用
```
