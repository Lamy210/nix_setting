# Design: Managed Nix Bootstrap

## Context

ADR-0001 で NixOS/nix-installer (LGPL-2.1) を SchneeForge の Managed-Nix provider に選定した (Status: provisionally accepted, macOS aarch64 smoke が final acceptance 条件)。

SchneeForge Core (Rust) は nix-installer を外部プロセスとして呼び、`/nix/receipt.json` を source of truth とする。GUI は Tauri IPC で Core を呼ぶが、Phase 1 では CLI のみ実装し、`schneeforge nix install` が完璧に動くことを先に保証する。

## Goals / Non-Goals

### Goals
- `schneeforge nix install` が fresh host (Linux x86_64, macOS aarch64) で冪等に動く
- `schneeforge nix uninstall` が nix-darwin 残留時でも安全に動く
- `schneeforge nix doctor` が receipt + 環境診断を統合して返す
- bootstrap-manifest.toml で version と SHA256 を pin し、runtime は online でverify
- supply-chain: release bump CI が SLSA attestation + SHA256SUMS を検証して manifest を bump

### Non-Goals (Phase 1)
- DMG / Homebrew formula 等の binary bundle (LGPL 再配布の法務設計は別 ADR)
- Tauri GUI 接続 (CLI 安定後、Phase 2)
- offline 配布用の事前キャッシュ配信サーバ (アプリデータ配下キャッシュのみ)
- Nix version を UI から直接指定する機能 (installer tag で間接指定)

## Decisions

### D1: Core に `managed_nix` module を新設し、Tauri 依存を入れない

```
crates/core/src/managed_nix/
  mod.rs
  provider.rs     // NixOS/nix-installer 固定の URL・arch map
  manifest.rs     // bootstrap-manifest.toml の version + sha256
  download.rs     // reqwest ベース。tauri-plugin-http 不使用
  verify.rs       // SHA256 / SLSA 検証
  installer.rs    // subprocess 実行・stderr JSON Lines parse
  receipt.rs      // /nix/receipt.json の読み取り専用 view
```

Rationale:
- Tauri への依存を Core から排除することで、CLI と GUI で同一実装を再利用できる
- `tauri-plugin-http` を Frontend に持たせると任意 URL 取得権限が Frontend に渡り、bootstrap manifest validation を Core に閉じ込められない
- reqwest は Rust workspace の標準的な HTTP client

### D2: Distribution は online download のみ (Phase 1)

URL は `https://github.com/NixOS/nix-installer/releases/download/{tag}/nix-installer-{arch}`。

Arch map:
| target | asset |
|--------|-------|
| x86_64-linux | `nix-installer-x86_64-linux` |
| aarch64-linux | `nix-installer-aarch64-linux` |
| aarch64-darwin | `nix-installer-aarch64-darwin` |
| x86_64-darwin | (本家未提供、Phase 1 非サポート) |

取得後、アプリデータ配下 (`XDG_DATA_HOME/schneeforge/managed-nix/{version}/nix-installer`) にキャッシュし、二回目以降の install は offline で動く。

### D3: Supply-chain は 2 層構造

**Release bump CI** (`.github/workflows/upstream-nix-installer.yml`, 週次 or 手動):
1. upstream の最新 release tag を取得
2. `gh attestation verify` (SLSA provenance) で release を verify
3. release の `SHA256SUMS` を取得し、対象 arch の expected sha256 を抽出
4. `bootstrap-manifest.toml` を更新する PR を自動作成
5. **bump PR は即時 merge ではなく、SchneeForge の release cycle で評価して merge する** (latest 追従ではない)。breaking change (CLI flag 廃止・receipt schema 変更等) がある場合は bump PR を棄却し、SchneeForge 側で対応を整えてから取り込む。
6. SLSA provenance 検証が失敗した場合は CI job を fail させ、`gh issue create` で tracked-issue を自動起票して手動対応を促す (Slack 通知等は Phase 2 以降)。

**Runtime verify** (利用者 PC):
1. `bootstrap-manifest.toml` の `version` と `sha256` を読む
2. download binary の SHA256 を計算
3. manifest の SHA256 と比較 (不一致は即座に abort)

利用者 PC で `gh` / `cosign` は不要。

### D4: Execution は subprocess。logger は stderr を JSON Lines parse

CLI 引数は以下の構文を取る (実測: `src/cli/subcommand/install/mod.rs`):
- `nix-installer install [PLANNER-SUBCOMMAND] [FLAGS]` — planner 指定で新規 plan を作って install
- `nix-installer install --plan <plan.json> [FLAGS]` — pre-built plan.json を読み込んで install
- `--plan` と planner-subcommand は排他 (両方指定で error)
- flags (`--no-confirm`, `--enable-flakes` 等) は `--plan` 利用時にも有効

SchneeForge は 2 段階 Plan UX (D7) のため `plan --out-file` → ユーザー確認 → `install --plan` を基本とする。

```
schneeforge nix install
  ↓
[Phase: Preflight (root 不要)]
  - /nix, nix-daemon, build users, shell profiles, flakes を変更する旨を表示
  ↓
[Phase: Download + Verify]
  - manifest の version + sha256 を読む
  - キャッシュが無ければ download
  - SHA256 を verify
  ↓
[Phase: Privilege escalation]
  - Phase 1 (CLI): 自前での sudo 呼び出しは行わず、root 未実行の場合は
    "sudo schneeforge nix install ..." で再実行を促して終了 (TTY 問題を避ける)
  - Phase 2 (GUI): privileged-gui-operations で osascript (macOS) /
    pkexec (Linux) を統合 (別 change で設計)
  ↓
[Phase: Plan]
  - nix-installer plan linux|steam-deck|ostree --out-file plan.json --enable-flakes
  - planner は SchneeForge が OS から選択 (linux/macos)
  ↓
[Phase: Install]
  - nix-installer install --plan plan.json --logger json --no-confirm
  - stderr を JSON Lines で best-effort parse
  - SchneeForge 側で大きな phase を進行表示
  ↓
[Phase: Post-install verification]
  - /nix/receipt.json を読んで整合性確認
  - nix run nixpkgs#hello 等で self-test
```

SchneeForge 側で管理する phase (Download / Verify / Privilege / Plan / Install / Post-install) を優先表示し、installer 内部の `Step: CreateUsers` / `Step: ConfigureNix` 等は詳細 progress として best-effort 表示。installer 内部メッセージの schema は unstable である前提で、schema 変更で SchneeForge が壊れないようにする。

### D5: Receipt は `/nix/receipt.json` を source of truth

SchneeForge 側で receipt を複製しない。読み取り専用 view (`Receipt { version, actions, planner }`) を Core が持ち、`schneeforge nix doctor` と `schneeforge nix uninstall` の入力にする。

### D6: Uninstall の順序保証

```
schneeforge nix uninstall
  ↓
ownership / safety check
  - /nix/receipt.json の存在確認
  - nix-darwin 検出
  ↓
nix-darwin が存在?
├─ YES → 警告を表示して abort (Phase 1 暫定)
│         「先に nix-darwin を外してください。SchneeForge は現在
│           nix-darwin の自動取り外しをサポートしません」
│         (nix-darwin の安全な取り外し手順は ADR-0001 Open Question 4。
│          別 change で設計後に自動化へ昇格)
└─ NO
  ↓
/nix/nix-installer uninstall --no-confirm
  - upstream の revert logic に委任
  ↓
cleanup 確認
  - /nix の削除、build users の削除を確認
```

### D7: doctor コマンド体系 (重複回避)

`schneeforge doctor` (既存) と `schneeforge nix doctor` (新設) は**役割を分離**し、`schneeforge doctor` から `schneeforge nix doctor` を呼び出す構造にする。

```
schneeforge doctor (既存: SchneeForge 全体診断)
  ├─ repository state (clone / dirty / ahead)
  ├─ manifest validation
  ├─ toolchain (nix / git / brew の解決状態)
  ├─ managed nix (↓ schneeforge nix doctor へ委譲)
  └─ apply state (current / desired / applied)

schneeforge nix doctor (新設: Managed Nix 専用)
  ├─ /nix/receipt.json の有無・整合性
  ├─ nix store ping
  ├─ nix config show experimental-features (flakes 有無)
  └─ receipt 内の actions 一覧
```

利用者は `schneeforge doctor` を使えば全体が見え、`schneeforge nix doctor` で Managed Nix 由来に絞れる。`schneeforge doctor` の nix 関連 section は `schneeforge nix doctor` の結果を埋め込む形で重複を避ける。

### D8: 2 段階 Plan UX

`nix-installer plan` は root を要求する (`ensure_root()` → sudo で再 exec) ため、GUI で「Plan を見る → いきなり sudo 認証」になるのを避ける。

```
SchneeForge preflight (root 不要)
  ↓
「Nix をインストールすると以下を変更します」
  - /nix
  - nix-daemon (Linux) / launchd (macOS)
  - build users
  - shell profiles
  - flakes
↓
[Continue]
↓
管理者認証 (Phase 1 CLI は root 再実行を促す、D4 参照)
↓
nix-installer plan linux|macos --out-file plan.json --enable-flakes
↓
Detailed Plan (actions 列を人間可読で表示)
↓
[Install]
  → nix-installer install --plan plan.json --logger json --no-confirm --enable-flakes
```

### D9: License の扱い

- SchneeForge と nix-installer は別プロセス (subprocess + pipes)。GPL/LGPL FAQ における "separate program" に該当し、SchneeForge 側コードにはライセンス伝染なし。
- LGPL-2.1 の dynamic-link exception の話ではない (link していないため)。
- binary を DMG 等へ bundle する再配布は Phase 1 では行わない。bundle 配布は別 ADR / 法務設計で扱う。

### D10: `ManagedNixError` enum

```rust
pub enum ManagedNixError {
    UnsupportedArch { arch: String },
    ChecksumMismatch { expected: String, actual: String },
    NetworkRequired,
    ReceiptNotFound { path: PathBuf },
    Download { source: reqwest::Error },
    Subprocess { exit_status: ExitStatus, stderr_tail: String },
    ManifestParse { source: toml::de::Error },
    PlanFileNotFound { path: PathBuf },
    PlannerConflict,
    ExistingNixDetected { path: PathBuf },
}
```

各 variant は SchneeForge の structured error (`crates/core/src/error.rs`) へ変換され、CLI は人間可読メッセージ + exit code を出す。spec.md の scenario 中に現れるエラー名 (`UnsupportedArch`, `ChecksumMismatch`, `NetworkRequired`, `ReceiptNotFound`) は全てこの enum の variant に対応する。

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| macOS aarch64 の実測が未完了 | ADR Status を provisional とし、Final acceptance に smoke test を必須条件とする |
| installer 内部メッセージの schema 変更 | D4: SchneeForge 側 phase を優先、installer メッセージは best-effort |
| offline 環境で初回起動が動かない | D2: アプリデータ配下キャッシュで 2 回目以降は offline 動作可能。完全 offline は Phase 2 |
| upstream の SLSA supply-chain が壊れた場合 | D3: Release bump CI が `gh attestation verify` で検知。runtime は pinned SHA256 で保護 |
| 利用者が OS 既存の Nix を上書きされる | installer は既存 Nix 検出時にデフォルトで停止。SchneeForge doctor が preflight で警告 |
| x86_64-darwin が未サポート | Phase 1 対象外を README と CLI で明示。Phase 2 で判断 |

## Migration Plan

### Phase 1 (本 change の範囲)
1. `crates/core/src/managed_nix/` モジュール新設
2. `bootstrap-manifest.toml` を commit (初期 version 2.35.1)
3. `schneeforge nix install / doctor / uninstall` CLI 実装
4. `.github/workflows/upstream-nix-installer.yml` (release bump CI)
5. `bootstrap-flow` spec の「Nix 未検出時のメッセージ」を curl|sh → Managed Nix へ更新
6. Linux x86_64 smoke test 通過
7. **macOS aarch64 smoke test 通過** (ADR-0001 Final acceptance)
8. ADR-0001 Status を `Accepted` へ昇格

### Phase 2 (別 change)
- Tauri GUI (First Run Wizard) への IPC 接続
- privileged-gui-operations と統合 (macOS 管理者認証)
- DMG bundle 配布の法務設計 (別 ADR)

### Rollback
- Phase 1 は CLI 追加のみで既存機能への影響無し。unsafe だと判明した場合は `managed_nix` module を feature flag で無効化可能。

## Open Questions

1. macOS aarch64 smoke 結果 (ADR-0001 Final acceptance の条件)
2. README "Stable (see note)" の note 行方 → 本家 issue で確認
3. nix-installer ↔ nix-darwin の install 順序 (Nix 先か nix-darwin 先か)
4. nix-darwin の安全な取り外し手順 (ADR-0001 Open Question 4 と重複)。Phase 1 は D6 の通り警告のみ。
5. Phase 2 の macOS GUI 認証方式 (osascript / STAuthorizationTool / pkexec) は privileged-gui-operations で別途決定
