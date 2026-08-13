# SchneeForge GUI 正常化・初回セットアップ改善 仕様書

**Status:** Proposal  
**Target:** GUI を「動く installer」へ。縦方向の integration（Frontend → IPC → Core → Nix）を完成させる

## 本質

今回の問題は単一バグではなく、以下が縦方向に未接続なことが本質。

```
Frontend init → Tauri IPC → DOM action → Status diagnostics
  → Tool discovery → Repository discovery → Bootstrap → Nix execution → State
```

`schneeforge-core` へロジックを集約した方向は正しい。次は「GUI から Nix まで」をつなぐ。

## 現状 (本仕様書作成時点)

- HEAD: `7a7d08b`
- CLI/Nix 構成: 完成度高い
- GUI backend: 基本機能あり（`get_status`/`run_scan`/`run_apply`/`run_rollback`/`run_upgrade`）
- GUI frontend: P0 バグあり（一部修正済み）
- Fresh install: 設計未完成
- GUI E2E: 不在

## 完了済み (P0 の一部)

| 項目 | 状態 |
|------|:---:|
| `withGlobalTauri: true` 追加（`window.__TAURI__` が undefined になる問題） | ✅ |
| ボタン ID と label の分離（`document.getElementById("スキャン")` → null の回帰） | ✅ |
| `CommandOutput.success` の処理（失敗時に error 表示） | ✅ |

---

## P0: 残る即時修正

### P0-A: frontend IPC guard

`window.__TAURI__` が無い場合に例外で固まらないよう guard を追加。

```js
if (!window.__TAURI__?.core?.invoke) {
  $("output").textContent = "Tauri APIを初期化できませんでした。";
  throw new Error("Tauri API unavailable");
}
```

### P0-B: devUrl 整理

現在 `tauri.conf.json` に `devUrl: "http://localhost:1420"` があるが、Vite dev server は使っていない（static index.html のみ）。`frontendDist` のみに整理。

---

## P1: 診断・依存解消

### P1-A: Status を診断 API に拡張

現状の Status（`host/user/nix/homebrew/git/applied_revision`）では「なぜ `-` か」が分からない。

```rust
struct Status {
    host: String,
    os: String,
    arch: String,
    repo_path: String,
    repo_exists: bool,
    manifest_found: bool,
    manifest_error: Option<String>,
    user: Option<String>,
    nix: ToolStatus,
    nh: ToolStatus,
    git: ToolStatus,
    homebrew: ToolStatus,
    state_path: String,
    state_found: bool,
    applied_revision: Option<String>,
    applied_at: Option<String>,
}

struct ToolStatus {
    available: bool,
    path: Option<String>,
    version: Option<String>,
}
```

### P1-B: GUI 表示を診断型に

```
System
  Host        macbook-air     ✓
  OS          macOS arm64     ✓
Configuration
  Repository  ~/nix_setting   ✕ Not found
  Manifest    config.toml     ✕ Not found
  User        -               ⚠
Runtime
  Nix         /nix/.../nix    ✓
  Git         /usr/bin/git    ✓
  nh          Not found       ✕
  Homebrew    /opt/.../brew   ✓
State
  Last Apply  Never
```

### P1-C: `~/nix_setting` の暗黙前提を解消（First Run Wizard）

DMG fresh install では repo が存在しない。存在しなければ通常画面ではなく Setup フローへ誘導:

```
Welcome → Scan → Configuration Setup → Plan → Apply → Verify
```

1. OS/arch 検出
2. Nix/Git 検出
3. repository 存在確認
4. clone/init
5. username 検出・確認
6. config.toml 生成
7. plan → apply → verify

### P1-D: ToolResolver（PATH 依存の解消）

macOS GUI は Terminal と同じ PATH を継承しない。`core` に統一的解決層を追加:

```
ToolResolver::nix() / nh() / git() / brew()
解決順: PATH → /nix/var/nix/profiles/default/bin → ~/.nix-profile/bin → /opt/homebrew/bin → /usr/local/bin
```

### P1-E: `nh` への bootstrap 循環依存を解消

現状: macOS は `nh darwin switch`。しかし `nh` は SchneeForge の Home Manager package で導入される。

```
fresh machine:
  SchneeForge 適用には nh が必要
  nh 導入には SchneeForge 適用が必要
  → 循環
```

修正: **core は nh に依存しない**。

- macOS: `nix run nix-darwin -- switch --flake <repo>#macbook-air`
- Linux: `homeConfigurations.*.activationPackage` を build + activate

`nh` は「環境構築後に使える便利 CLI」へ降格。

### P1-F: Tauri command の非同期化

重い操作（apply/rollback/upgrade/verify）は `tauri::async_runtime::spawn_blocking` で非同期化し、UI スレッドを占有しない。

```rust
#[tauri::command]
async fn run_apply() -> CommandOutput {
    tauri::async_runtime::spawn_blocking(move || {
        apply_captured(detect_host(), &resolve_repo(None))
    })
    .await
    .map_or_else(|e| CommandOutput { success: false, output: e.to_string() },
                 |r| match r {
                     Ok(out) => CommandOutput { success: true, output: out },
                     Err(e) => CommandOutput { success: false, output: e },
                 })
}
```

### P1-G: GUI 専用 state machine

```
Booting / Scanning / NeedsSetup / Ready / Planning
Applying / Verifying / RollingBack / Failed
```

- `NeedsSetup` → Apply ボタン非表示、`Set up SchneeForge` のみ表示
- `Applying` → mutating action を全 disable

---

## P2: 後続

### P2-A: GUI streaming output

Tauri event/channel で Rust process の stdout を逐次表示。

```
Rust process stdout → Tauri event → Frontend log viewer
```

### P2-B: GUI E2E テスト

| テスト | 成功条件 |
|--------|---------|
| App boot | window 起動 |
| Status | host が `-` でない |
| Refresh | IPC 成功 |
| Scan | output 変更 |
| Missing repo | 分かりやすい警告 |
| Apply mock | spinner 表示 |
| Action finish | spinner 消える |
| Backend error | error 表示 |
| Button | 実行中 disable |
| Unsupported platform | crash しない |

### P2-C: package/profile editor

---

## テストピラミッド

```
Core unit tests (42)     ← 多い
CLI integration (7)
Tauri command integration  ← 不足
GUI E2E                    ← 不足
Real macOS smoke           ← 少数
```

## 推奨実装順序

| Phase | 内容 | 優先度 |
|-------|------|:---:|
| GUI-0 | withGlobalTauri / IPC / button ID / success | ✅ 完了 |
| GUI-1 | IPC guard + devUrl 整理 | P0 |
| GUI-2 | Status diagnostics 拡張 | P1 |
| GUI-3 | First Run Wizard (repo なし対応) | P1 |
| GUI-4 | ToolResolver | P1 |
| GUI-5 | nh bootstrap 依存除去 | P1 |
| GUI-6 | async Tauri commands | P1 |
| GUI-7 | state machine | P1 |
| GUI-8 | GUI E2E | P1 |
| GUI-9 | streaming log | P2 |

## Release Gate (v0.2.0-rc)

```
SchneeForge.app 起動
→ Host macbook-air
→ Repository 表示
→ Nix Installed
→ Scan 成功
→ Plan 成功
→ Apply (spinner + log)
→ Verify 成功
→ Restart → applied revision 表示
→ Rollback 成功
```

## Fresh machine test (最重要ケース)

| ケース | 前提 | 検証 |
|--------|------|------|
| A: 既存利用者 | Nix/repo/config/nh あり | 全機能 |
| B: 新規利用者 | Nix のみ or fresh | Setup フロー → 全機能 |

「installer」として提供するなら **ケース B が最重要**。

## 設計原則

| 原則 | 方針 |
|------|------|
| KISS | Vanilla frontend を当面維持（React/Vite 不要） |
| DRY | CLI/GUI の実処理は core |
| SRP | discovery / tool resolve / actions / state を分離 |
| SOLID | GUI は core の adapter |
| YAGNI | React 等はまだ不要 |
| Diagnosability | `-` ではなく原因を表示 |

## リリース判断

- CLI/Nix: RC 候補
- GUI/DMG: 安定版 installer にはまだ早い → P0/P1 解消後に `v0.2.0-rc.1`
