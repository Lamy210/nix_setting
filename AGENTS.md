# AGENTS

SchneeForge（Declarative Developer Workstation Manager）の開発ルール。

## 開発フロー: OpenSpec を必ず使う

機能追加・変更はすべて OpenSpec の spec-driven ワークフローで行う。

```bash
# 1. 現状確認
openspec list                    # 進行中の change 一覧
openspec status --change <name>  # アーティファクト進捗

# 2. 新しい変更を提案
openspec new change <kebab-case-name>
# → openspec/changes/<name>/ が作成される
# → proposal.md → design.md → specs/ → tasks.md の順に書く

# 3. 検証
openspec validate <name>

# 4. tasks.md の順に実装（チェックを付ける）

# 5. 完了時にアーカイブ
openspec archive <name>
```

## ルール

- 手書きの `docs/*.md` spec は作らない（OpenSpec の changes/ を使う）
- 各 capability は `openspec/specs/<capability>/spec.md` に書く
- requirement には SHALL/MUST を、Scenario には WHEN/THEN を必ず含める
- `openspec validate` が通るまで実装を始めない

## アーキテクチャ

```
schneeforge-core (crates/core)   ← 実ロジック唯一の置き場
  ├── actions     (apply/rollback/scan/upgrade)
  ├── discovery   (detect_host/tool検出)
  ├── manifest    (config.toml)
  ├── repo        (repository解決)
  ├── state       (state.json)
  └── time        (時刻)
CLI (crates/cli)                 ← core を呼ぶだけ
Desktop (apps/desktop)           ← Tauri 2。core を呼ぶだけ
```

## 技術スタック

- Nix (flakes, flake-parts) / Home Manager / nix-darwin
- Rust: schneeforge-core / cli
- Tauri 2: desktop GUI
- 配布: flake / install.sh / GitHub Release (binaries + DMG) / Homebrew / cargo install

## 品質ゲート

- `cargo test` / `cargo clippy -- -D warnings` / `cargo fmt -- --check`
- `nix flake check` / statix / deadnix / actionlint / shellcheck
- コミット前にローカルで CI 相当を実行

## 現在進行中

- `openspec/changes/gui-normalization/` — GUI を動く installer へ（tasks.md 25件）
