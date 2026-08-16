# nix_setting

<img src="assets/icon.svg" width="96" height="96" alt="SchneeForge logo">

Nix + Home Manager + nix-darwin によるクロスプラットフォーム開発環境基盤。

- **対象**: Apple Silicon Mac / Linux x86_64 / Linux aarch64
- **管理**: パッケージ・dotfiles・macOS設定・Homebrew cask を宣言的に管理
- **one-line bootstrap (`install.sh`)**: Apple Silicon Mac / Linux x86_64 のみ
  (Linux aarch64 の release binary は未提供。Nix/Home Manager 設定自体は aarch64 Linux に対応)
- **検証**: CI で評価・ビルド・lint・secret scan を自動実行

> 開発への参加は [CONTRIBUTING.md](./CONTRIBUTING.md) を参照（ブランチ運用・OpenSpec・PR ルール）。

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

### Managed Nix (推奨)

Nix 未導入の machine では SchneeForge CLI 経由で Nix を install する
(version pinning・SHA256 検証・uninstall 時の ownership check 有効)。

```bash
# 1. クローン (OS/arch を自動検出)
git clone https://github.com/Lamy210/nix_setting.git "$HOME/nix_setting"
cd "$HOME/nix_setting"

# 2. schneeforge CLI を取得 (GitHub Release の binary または cargo build)
#    Release binary の場合 (Nix / Rust 環境不要):
#      v0.1.0 以降から schneeforge-<os>-<arch> を download して実行権限を付与
#    cargo build の場合 (Rust 環境が必要):
cargo build --release -p schneeforge

# 3. Managed Nix install (NixOS/nix-installer を SchneeForge が subprocess 実行)
#    (cargo build した場合は ./target/release/schneeforge を指定)
sudo ./target/release/schneeforge nix install
./target/release/schneeforge nix doctor   # /nix/receipt.json + nix store ping + flakes 確認

# 4. dotfiles 適用
./bootstrap.sh
```

### ワンライナー

**Stable** — release tag 時点の install.sh (CLI binary の pin 先と同一 release):

```bash
curl -fsSL https://raw.githubusercontent.com/Lamy210/nix_setting/v0.2.0-rc.5/install.sh | bash
```

**Edge** — main HEAD の install.sh (開発追従用):

```bash
curl -fsSL https://raw.githubusercontent.com/Lamy210/nix_setting/main/install.sh | bash
```

Stable は script の取得元 tag と `SCHNEEFORGE_BOOTSTRAP_VERSION` (CLI binary /
config ref の pin 先) が一致する release unit として動く。Edge は script 自体は
最新だが、download する CLI は install.sh 内の pin 値のまま (release 時に bump
される)。release 間で main の install.sh が古い pin のままの場合でも、pinned
release の asset が消えることはないため動作は継続する。

Nix 未導入環境では、この script が GitHub Release から schneeforge CLI binary を
download し (CHECKSUMS.txt の SHA256 で検証)、**Managed Nix 経路**で Nix を
install します (version pinning・ownership record 付き。ADR-0001 参照)。
既に Nix が導入済みの場合は何も install せず、flakes 有効化と
dotfiles 適用のみを行います。

Managed Nix は `bootstrap-manifest.toml` で version + SHA256 を pin し、
online で download + verify する。offline 配布・DMG bundle は Phase 2。
詳細: `docs/adr/0001-managed-nix-provider.md`

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

### Homebrew (macOS)

```bash
brew tap Lamy210/homebrew-tap
brew install schneeforge
```

### デスクトップ GUI (Tauri)

```bash
# DMG インストーラー (macOS)
# GitHub Release から SchneeForge_*.dmg をダウンロードして開く

# または Nix
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
- [docs/schneeforge-spec.md](./docs/schneeforge-spec.md) — SchneeForge 全体仕様書
- [docs/adr/0001-managed-nix-provider.md](./docs/adr/0001-managed-nix-provider.md) — Managed Nix provider 決定 (NixOS/nix-installer)
- [docs/spikes/2026-08-14-nix-bootstrap-provider-evaluation/spike-report.md](./docs/spikes/2026-08-14-nix-bootstrap-provider-evaluation/spike-report.md) — Provider 評価 Spike

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
