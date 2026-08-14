# Tasks: add-managed-nix-bootstrap

## 1. OpenSpec / ADR
- [x] 1.1 Spike Report (`spike-nix-bootstrap-provider-evaluation/spike-report.md`) の 3 点修正 (license, stderr, macOS coverage)
- [x] 1.2 ADR-0001 作成 (Status: provisionally accepted)
- [x] 1.3 本 change (proposal/design/tasks/spec deltas) 作成
- [ ] 1.4 `openspec validate add-managed-nix-bootstrap --strict` 通過

## 2. Core: `managed_nix` module (crates/core/src/managed_nix/)
- [ ] 2.1 `provider.rs`: NixOS/nix-installer の URL/asset 名を arch 毎に返す。`{ x86_64-linux, aarch64-linux, aarch64-darwin }` のみ (x86_64-darwin は未サポートで Err)
- [ ] 2.2 `manifest.rs`: `bootstrap-manifest.toml` の parse/serialize。schema は `[managed_nix] version, sha256_by_arch.{arch}`
- [ ] 2.3 `download.rs`: reqwest で asset を download。`XDG_DATA_HOME/schneeforge/managed-nix/{version}/nix-installer` へキャッシュ (存在時は skip)
- [ ] 2.4 `verify.rs`: SHA256 計算と manifest 比較。不一致は structured error (`ManagedNixError::ChecksumMismatch`)
- [ ] 2.5 `installer.rs`: subprocess 実行。`--logger json` の **stderr** を JSON Lines で best-effort parse し、`InstallPhase` enum (`Download / Verify / Privilege / Plan / Install / PostInstall`) に map
- [ ] 2.6 `receipt.rs`: `/nix/receipt.json` の読み取り専用 view (`Receipt { version, actions, planner }`)
- [ ] 2.7 `mod.rs`: `ManagedNix::install() / doctor() / uninstall()` の公開 API。`Toolchain` は既存を再利用

## 3. CLI: `schneeforge nix` subcommand (crates/cli/src/nix.rs)
- [ ] 3.1 `schneeforge nix install` — preflight → download → verify → privilege → plan → install → post-install
- [ ] 3.2 `schneeforge nix doctor` — receipt + `nix store ping` + `nix config show experimental-features`
- [ ] 3.3 `schneeforge nix uninstall` — nix-darwin 検出 → 順序保証 → upstream uninstall を subprocess
- [ ] 3.4 既存 `schneeforge doctor` から `schneeforge nix doctor` の結果を統合 (重複回避)

## 4. bootstrap-manifest.toml (repo root)
- [ ] 4.1 初期 manifest 作成 (version `2.35.1`, arch 毎の sha256 は Spike Report の `3b49a0b9…` 等を記載)
- [ ] 4.2 schema validation (`serde` + `toml`)

## 5. CI: upstream release tracking
- [ ] 5.1 `.github/workflows/upstream-nix-installer.yml` 新設 (weekly + 手動 dispatch)
- [ ] 5.2 `gh attestation verify` で SLSA provenance 検証
- [ ] 5.3 SHA256SUMS を取得し、`bootstrap-manifest.toml` を bump する PR を自動作成

## 6. Spec 修正
- [ ] 6.1 `bootstrap-flow` の「Nix 未検出時のメッセージ」要件を MODIFIED: curl|sh → Managed Nix アクション (`schneeforge nix install` を起動)

## 7. Test
- [ ] 7.1 unit: manifest parse, sha256 verify, JSON Lines parse (mock stderr)
- [ ] 7.2 integration: Docker container 上の fresh host (Linux x86_64) で `schneeforge nix install` → `doctor` → `uninstall`
- [ ] 7.3 **integration: macOS aarch64 disposable env で install / self-test / flakes / receipt / uninstall / cleanup** (ADR-0001 Final acceptance)
- [ ] 7.4 regression: `/nix/receipt.json` 存在時の冪等性 (2 回目 install は skip or up-to-date)
- [ ] 7.5 regression: nix-darwin 残留時の uninstall 拒否メッセージ

## 8. Docs
- [ ] 8.1 ADR-0001 Status を smoke 後に `Accepted` へ昇格
- [ ] 8.2 `docs/schneeforge-spec.md` の INSTALLER_DESIGN に Managed Nix を追記
- [ ] 8.3 README へ `schneeforge nix install` の簡単な使用方法を記載

## 9. Archive (PR 前)
- [ ] 9.1 `openspec validate add-managed-nix-bootstrap --strict` 通過
- [ ] 9.2 `openspec archive add-managed-nix-bootstrap` (specs へ反映後)
- [ ] 9.3 PR 作成 (`feat/managed-nix-bootstrap` → `develop`)
