# Tasks: add-managed-nix-bootstrap

## 1. OpenSpec / ADR
- [x] 1.1 Spike Report (`docs/spikes/2026-08-14-nix-bootstrap-provider-evaluation/spike-report.md`) の 3 点修正 (license, stderr, macOS coverage)
- [x] 1.2 ADR-0001 作成 (Status: provisionally accepted)
- [x] 1.3 本 change (proposal/design/tasks/spec deltas) 作成
- [x] 1.4 `openspec validate add-managed-nix-bootstrap --strict` 通過 (2026-08-14)
- [x] 1.5 レビュー指摘 (#1 plan.json 構文実測・#2 doctor体系・#5 ManagedNixError・#6 SLSA alert 具体化・#7 CLI privilege 方針・#8 ADR Alternatives A・#9 bump PR と release cycle・#10 nix-darwin 取り外し) を design.md / ADR-0001 / Spike Report へ反映

## 2. Core: `managed_nix` module (crates/core/src/managed_nix/)
- [x] 2.1 `provider.rs`: NixOS/nix-installer の URL/asset 名を arch 毎に返す。`{ x86_64-linux, aarch64-linux, aarch64-darwin }` のみ (x86_64-darwin は未サポートで `UnsupportedArch`)
- [x] 2.2 `manifest.rs`: `bootstrap-manifest.toml` の parse/serialize。schema は `[managed_nix] version, sha256_by_arch.{arch}`
- [x] 2.3 `download.rs`: reqwest で asset を download。`XDG_DATA_HOME/schneeforge/managed-nix/{version}/nix-installer` へキャッシュ (存在時は skip)
- [x] 2.4 `verify.rs`: SHA256 計算と manifest 比較。不一致は `ManagedNixError::ChecksumMismatch`
- [x] 2.5 `installer.rs`: subprocess 実行。CLI 引数は `install --plan <plan.json> --logger json --no-confirm --enable-flakes` を基本 (`--plan` と planner-subcommand は排他)。`--logger json` の **stderr** を JSON Lines で best-effort parse し、`InstallPhase` enum (`Download / Verify / Privilege / Plan / Install / PostInstall`) に map
- [x] 2.6 `receipt.rs`: `/nix/receipt.json` の読み取り専用 view (`Receipt { version, actions, planner }`)
- [x] 2.7 `mod.rs`: `ManagedNix::install() / doctor() / uninstall()` の公開 API。`Toolchain` は既存を再利用
- [x] 2.8 `error.rs` (または `mod.rs` 内): `ManagedNixError` enum 定義 (design.md D10 参照)。`crates/core/src/error.rs` の SchneeForgeError へ `From` 実装

## 3. CLI: `schneeforge nix` subcommand (crates/cli/src/nix_cmd.rs)
- [x] 3.1 `schneeforge nix install` — preflight → download → verify → privilege (root 未実行時は `sudo schneeforge nix install ...` で再実行を促す、D4) → plan → install → post-install
- [x] 3.2 `schneeforge nix doctor` — receipt + `nix store ping` + `nix config show experimental-features`
- [x] 3.3 `schneeforge nix uninstall` — nix-darwin 検出 → 残留時は警告で abort (D6) → upstream uninstall を subprocess
- [x] 3.4 既存 `schneeforge doctor` が `schneeforge nix doctor` を呼び出して nix 関連 section を埋める (D7)

## 4. bootstrap-manifest.toml (repo root)
- [x] 4.1 初期 manifest 作成 (version `2.35.1`, arch 毎の sha256 は upstream SHA256SUMS から取得)
- [x] 4.2 schema validation (`serde` + `toml`)

## 5. CI: upstream release tracking
- [x] 5.1 `.github/workflows/upstream-nix-installer.yml` 新設 (weekly + 手動 dispatch)
- [x] 5.2 `gh attestation verify` で SLSA provenance 検証
- [x] 5.3 SHA256SUMS を取得し、`bootstrap-manifest.toml` を bump する PR を自動作成 (即時 merge ではなく SchneeForge release cycle で評価、breaking 時は棄却)
- [x] 5.4 SLSA provenance 検証失敗時は CI fail + `gh issue create` で tracked-issue 自動起票

## 6. Spec 修正
- [x] 6.1 `bootstrap-flow` の「Nix 未検出時のメッセージ」要件を MODIFIED: curl|sh → Managed Nix アクション (`schneeforge nix install` を起動)

## 7. Test
- [x] 7.1 unit: manifest parse, sha256 verify, JSON Lines parse (mock stderr), `ManagedNixError` variant 毎の変換
- [ ] 7.2 integration: Docker container 上の fresh host (Linux x86_64) で `schneeforge nix install` → `doctor` → `uninstall`
- [ ] 7.3 **integration: macOS aarch64 disposable env で install / self-test / flakes / receipt / uninstall / cleanup** (ADR-0001 Final acceptance)
- [ ] 7.4 regression: `/nix/receipt.json` 存在時の冪等性 (2 回目 install は skip or up-to-date)
- [ ] 7.5 regression: nix-darwin 残留時の uninstall 警告 + abort メッセージ
- [x] 7.6 regression: root 未実行時に `sudo schneeforge nix install ...` の再実行を案内するメッセージ (CLI test 有)

## 8. Docs
- [ ] 8.1 ADR-0001 Status を smoke 後に `Accepted` へ昇格 (macOS aarch64 smoke が条件、Phase 1 範囲外)
- [x] 8.2 `docs/schneeforge-spec.md` の INSTALLER_DESIGN に Managed Nix を追記
- [x] 8.3 README へ `schneeforge nix install` の簡単な使用方法を記載

## 9. Archive (PR 前)
- [x] 9.1 `openspec validate add-managed-nix-bootstrap --strict` 通過
- [ ] 9.2 `openspec archive add-managed-nix-bootstrap` (specs へ反映後)
- [ ] 9.3 PR 作成 (`feat/managed-nix-bootstrap` → `develop`)
