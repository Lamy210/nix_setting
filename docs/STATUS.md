# STATUS（セッション引き継ぎ用）

現在の開発状態・既知のデグレ・機能漏れ・次の作業をまとめる。セッションを切り替えても、ここを読めば再開できる。

最終更新: 2026-11-05

## 2026-11-06 10:08+ JST: protected-branch integration and release gate checkpoint

- **Merged #287** Node security OpenSpec design-only to develop `63806e2a`; **merged #289** desktop serde_json patch-only `f807c439`; **merged #298** test-only ETXTBSY bounded retry `9147299d49ae30afae6adfebae98710d59de7cd6` after [22/22 current-head checks](https://github.com/Lamy210/nix_setting/actions/runs/42034731767). No production git retry behavior was changed.
- **Observed protected develop head when this snapshot was recorded:** `9147299d49ae30afae6adfebae98710d59de7cd6`. The [develop check run 42131602852](https://github.com/Lamy210/nix_setting/actions/runs/42131602852) failed because an existing OpenSpec `core-operations` dashboard channel scenario lacked a `THEN`; no required gate was bypassed.
- **PR #300** is a separate one-file specification-only fix, splitting source channel present/absent into two `WHEN/THEN` cases. As of snapshot [run 42132290765](https://github.com/Lamy210/nix_setting/actions/runs/42132290765) had 20/21 green, with macOS 26 Home Manager work still in progress. Merge only after every protected check passes.
- **PR #294** pinned Cachix install-nix-action 31.11.3 across five workflows; its non-force updated head `c8f278e821a62f89b1ee79d3c5a63dc09a2b425f` passes 20/22 but still fails the identical upstream-in-develop OpenSpec scenario. After #300 merge, resync non-force, rerun and require all checks before merge.
- **Draft PR #299** tests a separately locked Node 24.22.2 package (Brotli 1.2.1, c-ares 1.34.9) in root, independent Node template and independent devenv template; all three **Linux actual runtime probes passed** at [CI 42133132799](https://github.com/Lamy210/nix_setting/actions/runs/42133132799), with only OpenSpec + dependent aggregator red (20/22). These are version+Nix store closure-name checks, not independent source ABI proof or host remediation. A follow-up on #299 also adds macOS 15/26 actual runtime probes for all three shells; latest-head checks still required. Keep Draft until reviewed.
- **This STATUS PR #281** must remain after #300, #294 and a separately accepted #299 or explicit deferral; status changes must accurately reflect all latest protected commits. This section is a dated snapshot, not an assertion that CI will pass or all earlier historical runs were green.
- **RC.8 remains untagged and unpublished**. Pre-tag gates: Node #275 secure dependency provenance + all supported shell/host paths, Managed Nix safety and release supply-chain evidence, repo variable `SCHNEEFORGE_UPDATER_ACTIVATED` unset/false, release/* protected PR and current-head CI. Apple ID/Developer ID signing/notarization are not used; GUI updater remains disabled. Disposable Mac A–J remain post-tag *provisional RC* acceptance before stable endorsement. Latest public prerelease remains v0.2.0-rc.7, and releases/latest misclassification of rc.1 remains an independent task.

## 2026-11-05 evening JST: review and protected-branch integration handoff

- **Merged Node security design-only PR #287** into protected `develop` as `63806e2a849452a3c2794ed7c19306d68a654674`, after 22/22 successful latest-head checks. This does **not** upgrade actual Node binaries or close #275. The three independent Nix entrypoints and patched Brotli/c-ares provenance remain release blockers.
- **PR #289** (desktop `serde_json` patch in `apps/desktop/src-tauri/Cargo.lock` only) synced without force to `develop@63806e2` at `70e39378306501d6e1426091357a2e47ee14d0cc`. GitHub branch protection rejected the old-head merge as 7 required checks were newly expected; current-head CI [run 42021976828](https://github.com/Lamy210/nix_setting/actions/runs/42021976828) must pass before merge.
- **PR #294** (immutable `cachix/install-nix-action` SHA update across five workflow files) synced without force to `develop@63806e2` at `19da331485f0aa64194cfc1a03335034836cdbe8`; current-head CI [run 42022067223](https://github.com/Lamy210/nix_setting/actions/runs/42022067223) must pass before merge.
- **This STATUS PR #281** is documentation-only. Integrate it **after** #289/#294 or re-align its dated snapshot, and require its own current-head protected checks. No check bypass or force-push.
- **RC.8 is blocked**: no tag, no release asset, no deployment or host security fix is asserted. See [#271](https://github.com/Lamy210/nix_setting/issues/271) / [#275](https://github.com/Lamy210/nix_setting/issues/275). Apple Developer ID signing and notarization remain out of scope; production GUI updater activation must be verified disabled before any future prerelease tag.


## 2026-11-05: verified integration, repaired CI regression and security release gate

- **Current protected develop:** `820776074647a104a454a664892950233a5c036f`. [#288](https://github.com/Lamy210/nix_setting/pull/288) merged at `80acb9fc698e62dadc95acba1213f99021402f88`; unsigned RC policy [#286](https://github.com/Lamy210/nix_setting/pull/286) merged at `b9934074e8e5d5f882e35331e7a69ee3c69fd6b7` with **22/22 latest-head CI successes**; focused Rust bootstrap test fixture [#297](https://github.com/Lamy210/nix_setting/pull/297) was **squash merged** at `820776074647a104a454a664892950233a5c036f` after **22/22 latest-head CI** ([run 41918050153](https://github.com/Lamy210/nix_setting/actions/runs/41918050153)). #297 touches only the fake-git fixture, stages writer-closed executable then renames atomically; no production clone behavior changed.
- **Post-merge CI regression and fix:** [develop run 41916666197](https://github.com/Lamy210/nix_setting/actions/runs/41916666197) after #286, and earlier [STATUS #281 run 41917342140](https://github.com/Lamy210/nix_setting/actions/runs/41917342140), **failed** `rust-quality` on `bootstrap::tests::clone_repo_preserves_non_utf8_destination_bytes` with `fake-git: Text file busy (os error 26)` (410 pass, 1 fail); dependent `rust-check` failed closed. The same test passed with 411/411 in #297 and fmt/clippy succeeded. The previous failed runs are not retroactively green; check new protected develop and STATUS head separately before closure.
- **Node 24 security P0 is still open:** [#287](https://github.com/Lamy210/nix_setting/pull/287) OpenSpec **design only**, Ready for review, non-force synced to current `develop` as `cf3eb587e98cdbc2914ad5c5e3c9b47e71800710`, fresh CI pending (prior synced head passed 22/22). Root Nixpkgs lock `fe13242281a67641c0102cdb1087f0ebe586c7df` declares Node **24.21.0**, Brotli **1.2.0**, c-ares **1.34.8**; Node template independent lock `2fcb964de67fcf60b43471c55d5d99e61a9ccb5a` declares Node **24.18.1**, Brotli **1.2.0**, c-ares **1.34.8**. These are source recipe values, **not** evidence of active patched binaries or dependency closure. Devenv template lacks a committed lock and its `nix develop` runtime execution is skipped in current CI. Root + node + devenv must execute verified Node >=24.22.2 and patched shared/bundled Brotli/c-ares before [#275](https://github.com/Lamy210/nix_setting/issues/275) closure.
- **Other integrations pending:** desktop serde_json patch-only [#289](https://github.com/Lamy210/nix_setting/pull/289) non-force synced as `e9d3ab8501d97f280350075bfaf67b02a867f845`; pinned Nix Action 31.11.3 [#294](https://github.com/Lamy210/nix_setting/pull/294) non-force synced as `33e372a12ae760b41bd7d72bd77a6d3494f8a893`. Original synced heads had clean Rust quality and largely passing CI but latest **new heads require all protected checks before merge**. These remain separate from riskier #290/#291/#292/#293 and release/action major changes #295/#296; Draft #257/#278 deferred.
- **Release/host acceptance:** `v0.2.0-rc.7` is latest published prerelease; `v0.2.0-rc.8` is **not tagged**. Required Node P0, Managed Nix ownership/safety, current-head CI, attestation and explicit publish decision remain. The new unsigned/unnotarized release policy only permits a provisional/test-only prerelease after pretag gates; actual Apple Silicon Finder, install.sh/TTY and Managed Nix A–J must then pass before stable/accepted status. First-run bootstrap independent trust [#268](https://github.com/Lamy210/nix_setting/issues/268) remains open. Track [#271](https://github.com/Lamy210/nix_setting/issues/271).

## 2026-11-04: security integration / unsigned Preview / review handoff

- **Merged to `develop`:** owner-only machine input creation [#285](https://github.com/Lamy210/nix_setting/pull/285) → `02c53ecb2cac439f72a3242d22b9691bfc2991ad`; Rust Actions immutable SHA pinning [#283](https://github.com/Lamy210/nix_setting/pull/283) → `03ab82e6a96d70abe7476cfdec08a0af63325625`; review-only Dependabot policy [#282](https://github.com/Lamy210/nix_setting/pull/282) → `eb9aa548e39dbca52bcc49491241fa9dbcdcbed9`. Last #282 HEAD was 22/22 checks success; merge protected branch via squash, no force push.
- **Dependency update operational result:** Dependabot opened #288–#296 for both Cargo workspaces and GitHub Actions. This proves PR generation is enabled, not that all dependency updates are safe. Major jumps (e.g. `dirs` / `sha2` / update-flake-lock Action) require compatibility/security review; **do not automatically merge** or assume grouped patches are security remediations.
- **Apple-ID-free Preview:** [#286](https://github.com/Lamy210/nix_setting/pull/286) updated to HEAD `201228b0982551c39c7f3f49a42661f80be1f3a2` (latest CI pending). Release process distinguishes **tag-before public-test-only RC** (Node/Managed Nix P0 + required CI/artifact gates must already be green) from **post-tag Final Acceptance A–J**, run against that exact release's CLI / DMG / checksums. A–J incomplete → never call the RC accepted/stable; failures require a new RC. Apple Developer ID signing/notarization deliberately absent, Gatekeeper not automatically bypassed; `SCHNEEFORGE_UPDATER_ACTIVATED` must be verified false before candidate tag. Check docs `RELEASE.md` + `docs/testing/macOS-final-acceptance-checklist.md` in #286.
- **Node security spec:** [Draft #287](https://github.com/Lamy210/nix_setting/pull/287) updated HEAD `504b534a3e21efd44a0ac117e0cfac5a2acf82fd` (latest strict OpenSpec / CI pending): pins for root + standalone Node + currently-unlocked devenv; execute actual Node in each, assert Node 24 >=24.22.2 with reviewed dynamic/static Brotli/c-ares provenance. **No runtime fix merged**, P0 [#275](https://github.com/Lamy210/nix_setting/issues/275) remains open.
- **Other Drafts:** `flake.lock` [#257](https://github.com/Lamy210/nix_setting/pull/257) and provider `nix-installer 2.35.4` [#278](https://github.com/Lamy210/nix_setting/pull/278) are **deferred**; partial hosted macOS CLI lifecycle passing does not substitute Finder / `install.sh` / nix-darwin full A–J.
- **Security/supply chain:** GitHub Actions signatures/attestation are **cosign keyless + SLSA provenance / hashes**, *not* Apple Developer ID signatures or Apple notarization. Already published release remains `v0.2.0-rc.7`; `v0.2.0-rc.8` has **no tag**. First-run bootstrap trust issue [#268](https://github.com/Lamy210/nix_setting/issues/268) and release readiness [#271](https://github.com/Lamy210/nix_setting/issues/271) are open. Before any new RC tag, check Node P0, provider safety, supply-chain CI and production updater disabled. Only after that provisional release can A–J on exact release assets be attempted.

## 2026-11-03: Release readiness / Managed Nix 統合状況

- **開発統合:** PR [#273](https://github.com/Lamy210/nix_setting/pull/273) の Rust fake-git ETXTBSY 限定リトライを squash merge（`01ca1e5`）。PR [#272](https://github.com/Lamy210/nix_setting/pull/272) の RC 版案内・リリース条件を squash merge（`fb36865`）。PR [#259](https://github.com/Lamy210/nix_setting/pull/259) の read-only Nix 更新診断 **OpenSpec 設計のみ**を 26/26 CI で承認・squash merge（`162826b`）。
- **OpenSpec 実装待ち:** `add-managed-nix-upgrade-safety` Phase A `schneeforge nix upgrade inspect [--json]` は未実装。所有者、クライアント、daemon の確認は別々に行い、更新処理は Phase B の別承認。既存ホストのNix修復は未完了（[#256](https://github.com/Lamy210/nix_setting/issues/256)）。
- **Draft #278（新規インストール用 provider）:** `nix-installer 2.35.4` の pin と3アーキテクチャの SHA256 のみ変更。最新 HEAD `fdc10a6` で専用 Apple Silicon install→二重インストール拒否→uninstall→reinstall→final cleanup が [CI success](https://github.com/Lamy210/nix_setting/actions/runs/41579260712)。一般CI **27/27 checks 成功**。手動 macOS Final Acceptance A–J（Finder、install.sh/TTY、nix-darwin rollback）を待って **未マージ**。旧2.35.3 Draft [#244](https://github.com/Lamy210/nix_setting/pull/244) は後継への移行に伴いクローズ（未マージ）。
- **Draft #257（依存 lock）:** `home-manager`、`nix-darwin`、`nixpkgs` だけの `flake.lock` 更新。最新 HEAD `81acda6` で **26/26 checks 成功**。実機 activation と依存互換性の評価前に自動マージしない。
- **Node セキュリティ P0:** `nodejs_24` は現行の根元と node/devenv template の3経路で別々に解決。確認済み nixpkgs revision `1c89083e` は Node **24.21.0** のままで、24.22.1 security floor 未達。Node 24.22.2 / Brotli / c-ares の実行時 closure とホストを検証する（[#275](https://github.com/Lamy210/nix_setting/issues/275)、[#265](https://github.com/Lamy210/nix_setting/issues/265)）。
- **CI:** macOS preview Xcode 27.1 は一度 `/nix/var/nix/db/big-lock: Permission denied` で失敗し、その後の `develop` run は成功。原因未確定、失敗時の読み取り専用診断PR [#280](https://github.com/Lamy210/nix_setting/pull/280) は **26/26 checks 成功後に `c79af90` で develop へ squash merge 済み**（[#279](https://github.com/Lamy210/nix_setting/issues/279)）。週次 Lix mutable lock（[#276](https://github.com/Lamy210/nix_setting/issues/276)）、SBOM typepy mismatch（[#277](https://github.com/Lamy210/nix_setting/issues/277)）、Actions の自動 PR 作成権限（[#240](https://github.com/Lamy210/nix_setting/issues/240)）は未解決。
- **Release:** 公開プレビューは `v0.2.0-rc.7`。次の `v0.2.0-rc.8` は未タグ付け。Node P0・公開前CI/配布物ゲート合格までは RC を発行しない。公開したRCも実機 A–J と cosign/SLSA attestation の検証が完了するまでは正式版・受入済みとしない（Apple Developer ID 署名・公証とは別）（[#271](https://github.com/Lamy210/nix_setting/issues/271)）。`releases/latest` は過去の rc.1 が誤って非prerelease扱いのため rc.1 を返す状態が継続（Release metadata修正要）。

- **セキュリティ対応:** [#282](https://github.com/Lamy210/nix_setting/pull/282) はCargo×2・GitHub ActionsのDependabot週次更新ポリシー、[#283](https://github.com/Lamy210/nix_setting/pull/283) はRust GitHub Actionsの可変tag 15か所をSHA固定する提案。両PRは旧headで全CI成功し、#284マージ後の `develop` に同期して新head CI 検証中。CodeQL `rust/cleartext-logging` 対応 [#284](https://github.com/Lamy210/nix_setting/pull/284) は `schneeforge doctor` のユーザー名・HOME・hostnameを秘匿し、26/26 CI 成功後 `f8d6056` で squash merge 済み。machine input (`machine.nix`) を0600で作成する [#285](https://github.com/Lamy210/nix_setting/pull/285) は初回CIのテストimport・rustfmt不備を修正し、新head CI実行中。実際のSecurityタブopen alert一覧はGitHub連携から取得不可で、警告解消は未確認（[#265](https://github.com/Lamy210/nix_setting/issues/265)）。

## 2026-10-27: Managed Nix receipt marker の安全な状態分類（PR #258 マージ済み）

- `StatusProbe` は以前、receipt が存在しても JSON parse に失敗すると installation marker と見なさず `Missing` / `Broken` へ誤分類する場合があった。`schneeforge nix repair` が stale ownership record を誤削除するリスクのある条件を修正した。
- PR [#258](https://github.com/Lamy210/nix_setting/pull/258) は最終 HEAD `cd371b2` で **26/26 checks 成功**（`rust-quality`, `managed-nix-e2e`, macOS 15/26, `release-artifact-check` 等）を確認後、`develop` へ **squash merge**（`74b1905a66e0f8b21c27766c53a01b18f19e1458`）。破損 receipt + ownership、破損 receipt 単独、dangling symlink の回帰テスト3件を追加。
- 公開 `StatusProbe` に `receipt_exists` を追加したため、外部クレートが struct literal を直接作成する場合にはソース互換性への影響がある。リポジトリ内の呼び出しは更新済み。外部利用状況は unknown。
- PR [#259](https://github.com/Lamy210/nix_setting/pull/259) は `add-managed-nix-upgrade-safety` の **OpenSpec設計 Draft**。read-only 診断の提案承認前に新機能を実装しない。PR #259 のCIと実機の管理主体確認は別の gate。
- Issue [#256](https://github.com/Lamy210/nix_setting/issues/256) は未完了。PR #258 は既存Nix daemonの更新や脆弱性修正の実機適用を意味しない。
- PR [#261](https://github.com/Lamy210/nix_setting/pull/261) は 26/26 checks 成功後、`develop` へ squash merge（`eefb87629e5637330a52008808875e44c8d9aeb9`）。`/nix/store` と `/nix/var/nix` の dangling symlink を installation marker とみなし、`existing_nix_detected()` の再インストール拒否にも同じ fail-closed policy を適用。回帰テスト3件を追加。
- Draft #244 / #257 は引き続き release / compatibility acceptance 待ち。未承認の Draft を自動マージしない。

## 2026-10-23: CI / dependency update follow-up

- PR #251 (`fix(ci): respect closed installer bump PR decisions on reruns`) は最終 HEAD `ada7f18` の 26/26 checks が成功した後、`develop` へ squash merge 済み（`e869546`）。fork 同名 PR の誤認、CLOSED/MERGED PR の再作成、事前確認前の branch push を防止。
- Draft PR #248（2026-10-18 の `flake.lock` 回復）は HEAD `ec37bcf`、当時の `develop` との差分は `flake.lock` のみ。その後 PR #253 が `develop` にマージされたため、再同期と最新版の CI / upstream 互換性 / リリース受入確認が必要。Draft を維持してマージしない。
- `develop` push run [39811684901](https://github.com/Lamy210/nix_setting/actions/runs/39811684901) で `managed-nix-e2e` の Docker systemd 初期化に失敗（`setup_file`, `docker exec systemctl` exit 127）。musl release build は成功、12件の lifecycle tests は実行前に中止。真因は未確定であり flaky と判定しない。
- PR #253（Docker E2E 起動時の fail-fast・apt/systemd ログ診断・モック Docker Bats）は最終 HEAD `cf500975` で 26/26 checks 成功後、`develop` に squash merge 済み（`a46c77c`）。**既存 lifecycle gate の条件は緩和していない。** 以前の `develop` run `39811684901` の apt/systemd 真因は未確定。
- Issue #252 の macOS CI 計測基盤は PR #254（オフライン時間計測レポーター・7件の回帰テスト・入力データ収集手順）を最終 HEAD `c54a3e2` の 26/26 checks 成功後、`develop` へ squash merge 済み（`f9c5b53`）。追加レビューでは、複数時点で取得した同一 job ID の queued→completed スナップショットを先着優先で集計し、成功結果を落とすリスクを発見。後続 PR で completed 優先・矛盾する完了データの fail-closed・追加回帰テストを実施中。計測は高速化ではなく、成功実行10件以上・cache provenance が次の前提。
- Issue #240（定期依存更新 PR 作成の repository/org 権限）は未解決。Draft #244（nix-installer 2.35.3）は macOS lifecycle/配布受入完了までマージ禁止。


## 2026-10-21: repository path identity（実装完了・OpenSpec同期）

- 設計 PR #237、OpenSpec strict 検証修正 PR #241 (`03e750c`)、実装 PR #238 (`2ffcf1e`) を `develop` に squash merge 済み。
- Rust Core の `RepositoryPath` / typed `EffectiveSourceRef`、`OsString` 対応プロセス引数、CLI `--repo: PathBuf`、Desktop/Tauri の OS-native repository 解決に移行した。Linux の non-UTF-8 repository path と fake process argv を regression test で検証。
- PR #238 の最終 HEAD `505b968` は Actions run `39519506446` で required CI、Rust、OpenSpec、macOS 15/26、Windows、CodeQL、release-artifact を含め全ジョブ成功。OpenSpec strict 問題 #239 も PR #241 で修正・close 済み。
- 設計の migration step 8 に基づく旧 String-based `resolve_repo` / `resolve_repo_with` / `effective_ref` の撤去と公開 exports・テスト移行は PR #242 (`98e6c5e`) で squash merge 済み。PR #242 の最終 HEAD `ea7399e` では 22 CI jobs と 26 checks が成功。
- OpenSpec の `refactor-repository-path-identity` proposal/design/tasks/差分仕様は PR #243 (`e5ca7e1`) で archive 済み。canonical `core-operations` / `gui-diagnostics` / `managed-nix-bootstrap` を同期し、最終 HEAD `1130fda` の CI 22 jobs / 26 checks が成功。
- **意図的な制限**: upstream Nix の local flake parser が全 non-UTF-8 path を受理することまでは保証しない。GUI の AppleScript/pkexec による権限昇格は引き続き non-UTF-8 repo path を fail-closed で拒否する。
- 定期更新ワークフローのPR作成失敗時の診断と復旧案内は PR #245 (`c69f5f4`) で `develop` に squash merge 済み。Issue #240 のrepository/organization側PR作成権限は引き続き未解決。
- 更新ワークフローの再実行安全性: PR #246 (`a0907be`) を `develop` に squash merge 済み。nix-installer bump branch / 既存PR本文をforce-push・本文上書きから保護。manifest以外の差分、ハッシュ不一致、remote障害、非`develop` manual dispatchはfail-closed。最終HEAD `bd1ca2f` はCI 22/22ジョブ、26/26 checksが成功。
- CI follow-up: PR #247（installer lookup API 障害時の fail-closed）と PR #251（fork 同名 branch の誤認防止、閉鎖/マージ済みPR確認、push 前 preflight、Bats 回帰テスト）は `develop` にマージ済み。issue #240 の GitHub Actions による PR 作成の管理者側設定は未解決。
- リリース readiness: macOS Final Acceptance、production GUI updater activation の手動 gate は別途未完了。CI dependency-update の PR 作成権限問題は issue #240 で追跡中。既存 `bump/nix-installer-2.35.3` branch は Draft PR #244 として救済済み（release-cycle acceptance 未完了のためマージ禁止）。

## 完成済み

| 領域 | 内容 |
|------|------|
| Nix 基盤 | flake-parts / hosts / profiles / manifest / 3システム / CI 10+ジョブ |
| Rust core | actions / discovery / execution / diagnostics / manifest / repo / state / time / tool / lock / operations / process / bootstrap / self_update |
| CLI | native macOS/Linux adapter + experimental Windows launcher / WSL2 delegation |
| Tauri GUI | 診断 Status + First Run Wizard + 非同期コマンド + CSP + 状態機械 + Plan/Verify ボタン + `fix-path-env-rs` による macOS PATH 補正 |
| OpenSpec | `gui-normalization` / runtime hardening / managed source / release supply-chain / development workflow hardening 等を archive 済み。active change は下記「進行中」を参照 |

### gui-normalization（63/63 完了・PR #4 merge 済み）

- **Phase 0**: spec 整合
- **Phase 1**: Platform/Architecture/ConfigurationTarget 分離、`Manifest::validate`、`ToolResolver`、`RepoResolver`、structured error
- **Phase 2**: atomic `StateStore`、クロスプロセス flock ロック、apply/rollback の State 永続化
- **Phase 3**: repo-aware 操作（plan/apply/verify/rollback/upgrade/sync）+ CLI の core 委譲
- **Phase 4**: preflight、pinned nix-darwin bootstrap、nh 非依存 apply、権限/backup 設計
- **Phase 5**: 診断 API（`Diagnostics` / tool path/version / manifest 検証）
- **Phase 6**: desktop 非同期コマンド、CSP、opener 削除
- **Phase 7**: First Run Wizard（clone/config 生成/plan/confirm/apply/verify/resume）
- **Phase 8**: GUI E2E 計画 + action mapping 静的クロスチェック
- **Phase 9**: CI smoke（CLI/desktop）、Homebrew tap 分離、README/RELEASE 同期

### 追加で対応済み（gui-normalization 後）

| 項目 | PR |
|------|----|
| install.sh / bootstrap.sh の username 個人化（#1） | #5 merge 済み |
| config.toml 生成の冪等化 + username 空ガード（#2/#3） | #9 |
| Ready 画面 Plan/Verify ボタン（#13） | #6 |
| uninstall の副作用排除（#10） | #7 |
| archive-before-pr のドキュメント修正（当時のプロセス改善。2026-09-16 の workflow hardening で後続運用へ更新） | #8 |

### runtime-tool-resolution-hardening（実装完了・merge 済み #11）

P0-1〜P0-5 を 1 change に統合して実装。PR #11 (squash) で develop へ merge 済み。

- **P0-1 ToolResolver 強化**: `ToolSource` / `ResolvedTool` / `Toolchain` / `ToolchainError` 追加。8 段階の探索優先度（env → PATH → XDG state → Nix profile 群 → per-user → system profile → Homebrew）。`canonicalize` で symlink 解決。
- **P0-2 全操作の Toolchain 経由化**: `actions` / `operations` / `bootstrap` の全関数が `&Toolchain` 受け。`process` 系は `&str` → `&Path` へ型変更。
- **P0-3 Nix Health Check**: `NixHealth` struct 追加。`nix store ping` / `nix config show experimental-features` で実環境検証。
- **P0-4 Flakes 検出バグ修正**: `PreflightReport` を `{ nix_installed, flakes_enabled, git_installed }` へ分離。`nix config show` を使って正確に判定。
- **P0-5 install.sh / bootstrap.sh 探索統一**: `scripts/resolve-tools.sh` 新設。Rust 側と同一の探索順序。`tests/resolve-tools.bats` (11 ケース) で回帰テスト。
- **前提**: `fix-path-env-rs` 追加（macOS の Finder/Spotlight 起動時の PATH 欠損を補正）。
- **CI 再発防止**: `lint` job に "forbid raw tool spawns" step 追加。`tool.rs` / `cli/tests/` 以外での文字列リテラル spawn を禁止。
- **GUI 側**: `CachedToolchain` (`Mutex<Option<Toolchain>>`) を `tauri::State` で保持。
- **未完了**: macOS desktop build + Finder launch 実機検証は macOS Final Acceptance に統合。

### docs/release-checklist-and-tap-sync（merge 済み #12）

RELEASE.md のリリースチェックリスト再構築 + weekly workflow の権限修正。

### v2 P0/P1 + Manifest + profile + Release Metadata + GUI Dashboard (2026-08-18 merge 済み)

2026-08-18 に develop へ squash merge 済み (PR #43-#52)。詳細は各 archive 済み change を参照。

- **v2 P0/P1 (PR #43/#44/#45)**: MachineFacts / ConfigurationSource / archive 整備
- **Distribution Manifest (PR #46/#47)**: `schneeforge.toml` (schema 1 / profiles / systems) で旧 config.toml 置換
- **profile 選択の flake 注入 (PR #48/#49)**: flake input `profile` + `modules/profile-input.nix`、CLI `profile list/set/clear/show`
- **Release Metadata (PR #50)**: `schneeforge-release.json` parse/validate/fetch + CLI `source metadata`
- **GUI Dashboard §28 (PR #52/#53)**: core `dashboard.rs` + desktop `get_dashboard` + Dashboard card

### managed source 3 change + 初期化経路 unified (2026-08-20 merge 済み, develop a0bc0c5)

CLI / install.sh / GUI wizard の初期化経路が unified (v2 §7 完結)。

- **GUI profile 切替 (PR #56 / 0fddc8f + archive #58)**: core `ProfileList` / `set_selection`、desktop profile 操作、Dashboard profile 切替 UI
- **HOME env race 修正 (PR #57 / d6cd751)**: MachineFacts detect の env 差し込みで test race を解消
- **install.sh fresh 経路の managed source 化 (PR #59 / 77a4d20 + archive #61)**: release asset CLI → source init --tag pin → apply
- **GUI wizard の managed source 対応 (PR #60 / 60e2846 + archive #61)**: managed source を既定経路へ追加

### GUI source 更新 + test 隔離 + ドキュメント整備 + provenance (2026-08-21 merge 済み, develop d1f049a)

PR #62-#68 を sequential chain で merge。

- **GUI からの source 更新 (PR #62 / f0a880b + archive #68)**: `run_update` Tauri command +「ソース更新」ボタン
- **cli test の state/network 隔離 (PR #63 / a6621e2)**: XDG_STATE_HOME 隔離 + local origin 化
- **docs (PR #64-#66)**: STATUS / Final Acceptance / RELEASE checklist 更新
- **release asset provenance (PR #67 / a1fb376 + archive #68)**: `actions/attest-build-provenance` を追加

### v0.2.0-rc.7 release (2026-08-22)

**rc.7 を release 済み** (PR #77 / main f6a1f33 / tag v0.2.0-rc.7)。

- rc.6 は `attestations: write` 不足で release workflow が失敗し、rc.7 へ切り直し (PR #76)
- asset 6 個の CHECKSUMS 一致、provenance attestation subject を確認
- managed release source / `schneeforge-release.json` / SLSA provenance / CLI self-update を初同梱

### CLI 自己更新 self-update (2026-08-22 merge 済み, develop 653fc1f)

- `schneeforge self-update`: channel latest tag 解決 → release binary download → checksum verify → atomic replace
- core は純関数と effect を分離し hermetic test 化
- GUI 内自己更新は別 change。Step 1 Releases link は PR #81 で実装済み

### rc.7 後 follow-up (2026-08-23〜08-30)

- Homebrew tap を rc.7 へ更新 (`homebrew-tap#1`)
- RELEASE.md の tap 節を実態へ修正 (PR #80)
- GUI 自己更新 Step 1 Releases link button (PR #81)
- self-update UpToDate 誤表示 fix (PR #84)
- release attestation bundles 実装 PR #86 → archive/spec sync PR #87

### GUI 自己更新 Step 2 の設計判断確定 (2026-08-23)

- **鍵管理**: GitHub Actions secret + offline backup
- **Linux GUI**: updater 対象外
- **latest.json**: release workflow 自前生成
- **タイミング**: v0.3 で別 change

### 開発ワークフロー hardening（2026-09-16 archive 済み）

- topic → `develop` は squash merge、`release/*` → `main` と `main` → `develop` back-merge は merge commit に統一
- OpenSpec archive は実装 PR merge 後の separate `chore/archive-*` PR に統一
- `main` / `develop` の既存 diverged history は rewrite/force push せず、今後の ancestry を merge commit で維持
- `AGENTS.md` / `CONTRIBUTING.md` / `RELEASE.md` / OpenSpec workflow を同期

### CI critical-path optimization（2026-09-16 archive 済み）

- Rust required gate を `rust-quality` / `rust-build-smoke` の2 workerへ分割し、既存 required context `rust-check` は fail-closed aggregator として維持
- Linux desktop smoke は CLI sidecar と同じ release profile の `cargo check --release` に変更し、full DMG/Tauri build は required `release-artifact-check` に集約
- shadow `ci-required` を追加し、現行 required 7 contexts を fail-closed で集約（server-side branch protection は本 change では変更しない）
- Linux required `flake-check` は default `developer` profile を derivation evaluation、supported `minimal` profile を actual realization。product default は `developer` のまま
- baseline run #378: required critical path 479s / old `rust-check` 448s
- final measured run #397: current required critical path **327s（31.7%短縮）**、Rust runner total **443s**。目標 `<=359s` / `<=537.6s` をともに達成
- `flake-check` は 387s級のボトルネックから61sまで短縮。Terraform source build を毎PRのrequired Linux realizationから外しつつ、default developer evaluationは維持
- 実装 PR #90 と archive/spec-sync PR #92 はともに squash merge 済み

### macOS compatibility matrix（2026-09-16 archive 済み）

- stable compatibility lane: `macos-15` + Xcode 26.3、stable current/shipping lane: `macos-26` + Xcode 26.6
- `macos-check` は stable 2 laneをfail-closedで集約し、既存required contextsは変更していない
- `release-artifact-check` とtag releaseも `macos-26` + Xcode 26.6へpin
- Xcode 27はPR外のpreview canaryとして分離し、required/release dependencyには入れていない
- hosted runner が `macos-15-arm64` / `macos-26-arm64` であることも実runで確認済み
- latest-head run #428でrequired 7 contexts、`ci-required`、release artifact、stable 2 lane、`macos-check`が全てgreen
- old single-lane baseline 544sに対しstable 2 lane totalは約1227s（約2.26x）で、2.5x guard 1360s以内
- 実装 PR #93 と archive/spec-sync PR #94 はともに squash merge 済み

### Windows / WSL2 experimental backend（2026-09-18 merge / archive 済み）

- PR #96 で native Windows launcher + WSL2 delegation を実装し develop へ merge
- PR #98 で `add-windows-wsl2-platform` を archive + canonical spec sync
- PR #99 で Windows checkout 時の CRLF を `.gitattributes` で修正
- PR #100 で WSL delegation を `wsl.exe --exec` へ修正し shell-free argv contract を実境界で成立
- PR #101/#102 で real WSL2 canary harness と静的 contract を hardening
- real WSL2 canary は explicit selector / env selector / literal argv / delegated exit code を実 Windows→WSL2 で green 確認済み
- Windows release asset / coordinated self-update は initial scope 外で、experimental source-build 段階を維持


### macOS Managed Nix release lifecycle helper（2026-09-19 merge / real run 済み）

- PR #105 で manual-only / non-required の `macos-managed-nix-lifecycle.yml` と release-artifact lifecycle verifier を追加
- PR #109 で hosted macOS の synthetic `/nix` cleanup 判定を runtime/state remnants 基準へ修正
- PR #111 で flakes smoke を self-contained local flake に変更し、unauthenticated GitHub API rate limit 依存を除去
- production `develop@6bae1c21` から `v0.2.0-rc.7` を指定した workflow run #5 (`35435044077`) が success
- `macos-15-arm64` fresh host で release CLI checksum → Managed Nix install → receipt/ownership → store/local flake → doctor → `ExistingNixDetected` → uninstall → reinstall → final cleanup を実境界で確認
- これは CLI lifecycle の自動 evidence であり、Finder GUI / `install.sh` D8 / nix-darwin full bootstrap の manual Final Acceptance を置き換えない

### GUI self-update Step 2（2026-09-21 実装 merge 済み / production activation 未実施）

- PR #113 で macOS aarch64 向け Tauri 2 signed updater の実装準備を `develop` へ merge
- backend が pending update を保持し、`fetch_app_update` / `install_app_update` / progress event / restart を提供
- Dashboard は updater capability が有効な build のみ自動更新 button を表示し、GitHub Releases link を fallback として維持
- release pipeline は `SCHNEEFORGE_UPDATER_ACTIVATED=true` の場合だけ signed `.app.tar.gz` / `.sig` / `latest.json` を生成する staged activation
- PR #115 で updater signature を app version と暗号学的にbindし、runtime は signed version と endpoint announced version の一致を必須化（Tauri CLI 2.11.5+ / `requireSignedVersion=true`）
- production signing private key は Tauri build step のみに scope し、通常 PR/develop build は secret-free / updater-disabled
- **未完了**: macOS manual Final Acceptance、production key pair/secret/public-key provision、signed N→N+1 E2E、tampered artifact signature mismatch E2E
- 上記 activation gate 完了までは production updater を有効化せず、placeholder/test trust root は shipping しない

### Release/source fail-closed hardening + Managed Nix 2.35.2（2026-09-23〜09-30）

release/source/update 境界の重複実装と silent fallback を整理し、Managed Nix provider bump の実機相当 acceptance を pre-merge で閉じた。

- **SemVer / release metadata (#118-#121)**:
  - updater manifest / release metadata の release tag を strict SemVer として fail-closed 検証
  - release tag ordering を SemVer precedence 準拠へ修正し、巨大 numeric identifier / prerelease / build metadata を回帰テスト
  - core の tag selection / Dashboard update comparison / CLI self-update を共通 strict SemVer comparator へ集約
- **release channel fail-closed (#122/#129)**:
  - `stable` / `preview` 以外を stable へ暗黙 fallback しない
  - runtime channel は persisted free-form value ではなく release `SourceKind` から導出
- **managed source trust boundary (#123/#125/#127/#128/#130)**:
  - managed file cache を repository provenance + content digest に bind
  - missing state と corrupt/unreadable state を分離し、state-dependent effect 前に error を伝播
  - `managed=true` は immutable GitHub release source のみ許可し、mutable ref / kind-tag mismatch / unsupported remote を checkout fallback・network/cache effect 前に拒否
  - Diagnostics / Dashboard / source status / self-update も semantic corruption を未初期化扱いしない
- **2026-09-30 fail-closed follow-up (#138/#139)**:
  - checkout 型 `update()` の更新後 source 再検出 error を `None` へ変換せず伝播し、失敗時は既存 source/profile state を保持
  - `bootstrap-manifest.toml` の symlink 拒否を public `ManagedNix::load_from_repo()` 自体へ集約し、direct caller でも repository trust boundary を維持
  - #139 は #138 merge 後の `develop` を history rewrite 無しで topic branch に取り込み、latest-head run #701 の全 22 jobs green を確認して squash merge
- **upstream bump workflow hardening (#131)**:
  - `bootstrap-manifest.toml` 全書き換えを廃止し、専用 helper で version + 3 architecture SHA256 のみ更新
  - header comment / ordering / unrelated metadata を保持し、expected field の欠落・重複は fail-closed
- **Managed Nix provider 2.35.2 (#132)**:
  - NixOS/nix-installer `2.35.1 -> 2.35.2` を upstream diff / SLSA-verified bump / official asset SHA256 で評価
  - current pin: **2.35.2**
  - Linux x86_64 Docker full lifecycle（install / doctor / second-install rejection / uninstall / reinstall / repair）が green
  - Managed Nix E2E の installer version expectation は manifest 由来へ変更し、将来 bump の hard-code drift を除去
- **Managed Nix bump macOS acceptance (#133)**:
  - `bootstrap-manifest.toml` 等を変更する PR に限定した `macos-15` fresh-host acceptance workflow を追加
  - current branch の release CLI を Nix-less Apple Silicon runner 上で buildし、embedded manifestを使って install → receipt/ownership → store/local flake → doctor → `ExistingNixDetected` → uninstall → reinstall → final cleanup を自動検証
  - #132 の **2.35.2 branch binary** でも full lifecycle green を確認
  - published release tag を対象にする既存 manual lifecycle mode は維持

### Bootstrap / source-state fail-closed follow-up（2026-10-01〜10-03）

既存 OpenSpec の bootstrap / state / source safety 契約に対する audit を継続し、silent fallback と mutation 前検証漏れを修正した。

- **missing-Nix / flakes bootstrap (#141-#147)**:
  - missing Nix guidance を Managed Nix (`schneeforge nix install`) に統一し、legacy curl|sh guidance を除去
  - setup は flakes enable 後に再診断し、`nix-command` + `flakes` の両方が実効設定として確認できるまで apply へ進まない
  - Rust / install.sh / bootstrap.sh は実効 `nix config show experimental-features` を exact token で判定し、inspection failure は config mutation 前に fail-closed
  - `extra-experimental-features` で既存 feature を破壊せず、安全に newline を補って追記
- **read-only state/manifest surfaces (#142/#148/#150/#151)**:
  - CLI scan / Desktop Dashboard / CLI status・doctor / core verify が corrupt・semantic-invalid managed state を missing/unset/local fallback に変換しない
  - genuine missing local manifest の informational path は維持
- **Git/source classification and release update (#149/#152-#160)**:
  - Git branch/tag inspection error を detached/tagless 状態として扱わず、source init / sync / release update / deps warning まで error を伝播
  - apply / rollback 後も checkout source を保持し、Preview 等の release channel semantic を消さない
  - detached HEAD の複数 exact tags は ambiguous として拒否
  - release tag fetch failure は stale local tags へ fallback せず停止
  - release metadata は選択 repository に bind し、404 の legacy missing-asset 以外の fetch/validation error を fail-closed
  - release page version / repository URL、managed source revision、existing source-init state を canonical boundary で検証
- **state mutation / dependency update boundaries (#161-#163)**:
  - profile selection/clear は semantic-invalid managed state を上書きしない
  - deprecated `upgrade` は canonical `deps_update` safety boundary を経由
  - unmanaged Git checkout では Git が解決できない状態で source classification を省略して `nix flake update` へ進まない。Git 管理外 Local source は従来どおり Git 無しで更新可能
- `develop@42fcc900` で #163 を squash merge。run #801 は protected required 7 contexts、`ci-required`、`release-artifact-check`（DMG verify + mounted app `/nix/store` 非依存を含む）まで green。
- 2026-10-03 時点で open PR / active OpenSpec change は 0。コード上の既知 release blocker は無し。

## 進行中

| 項目 | 進捗 | 場所 |
|------|------|------|
| macOS Apple Silicon Final Acceptance | rc.7 の CLI lifecycle 自動化後も Finder GUI / install.sh 対話 / full bootstrap manual gate は未完了 | `docs/testing/macOS-final-acceptance-checklist.md` |
| GUI self-update Step 2 activation | 実装は merge 済み。production key provision / Final Acceptance / N→N+1・tamper E2E 完了まで disabled | `RELEASE.md`, `openspec/specs/gui-dashboard/spec.md`, `openspec/specs/release-supply-chain/spec.md` |
| DMG offline bundle 法務 ADR (issue #17) | ADR-0002 / OpenSpec は archive 済み。binary bundle / offline install 実装は弁護士確認後 | `docs/adr/0002-dmg-bundle-lgpl-redistribution.md`, `openspec/changes/archive/2026-09-17-add-dmg-offline-bundle-licensing/` |

## 既知のデグレ・機能漏れ（要対応）

### 高（Release Blocker）

現時点でコード上の既知 release blocker は無し。macOS Final Acceptance は release readiness gate として未完了。

### 中

| # | 問題 | 対応 |
|---|------|------|
| — | Dependabot alert #2: glib 0.18.5 (GHSA-wrw7-89jp-8q8g / RUSTSEC-2024-0429, medium) | Tauri v2 Linux → GTK3 0.18 → glib 0.18.5 経由。upstream stable release を monitor し、gtk3-rs stable 更新後に再評価。dismiss せず tracking 継続 |
| — | `main` と `develop` の Git history が diverge | **history rewrite しない**。次回 release から `release/*`→`main` と `main`→`develop` を merge commit 固定し、以後の ancestry を維持 |

### 低

| # | 問題 | 対応 |
|---|------|------|
| — | Windows release asset / coordinated self-update 未提供 | PR #96 initial scope外。launcher/helperを同一versionで手動用意するexperimental source-build段階。distribution/update設計をfollow-up changeで行う |
| — | バージョン文字列の同期（現在 `0.2.0-rc.7`） | 次回 release で RELEASE.md checklist に従い同期 |
| — | Intel macOS release asset 未提供 | `add-x86_64-darwin-support` を Windows/macOS compatibility 基盤後に検討 |

## 次の作業（推奨順）

1. **macOS Apple Silicon Final Acceptance の残り manual gate**
   - Finder pre-bootstrap / post-bootstrap GUI smoke
   - `install.sh` D8 `/dev/tty` 経路
   - nix-darwin apply 済み full uninstall ordering
   - 全 gate 完了後にのみ ADR-0001 を `Accepted` へ昇格
2. **GUI self-update Step 2 production activation**
   - Final Acceptance PASS 後に production updater key pair を生成し、private key/password を Actions secret + offline backupへ保管
   - public key を review して production config に固定
   - signed N→N+1 updater E2E と tampered artifact signature mismatch E2E を通してから `SCHNEEFORGE_UPDATER_ACTIVATED=true`
3. Phase 2/E follow-up
   - Windows release asset / coordinated launcher-helper update設計
   - Intel macOS release asset の検討
4. **issue #17**
   - LGPL-2.1 再配布条件の弁護士確認後に DMG offline bundle 実装を開始

※ issue #14/#15/#16/#91 は close 済み。#17 は法務確認待ち。

## 開発フロー

- Topic branch: `develop` → `feat|fix|refactor|docs|test|chore/*` → PR → `develop` (**squash merge**)
- Release: `develop` → `release/vX.Y.Z` → PR → `main` (**merge commit**) → tag → release workflow
- Back-merge: `main` → PR → `develop` (**merge commit**)。squash/rebase/force rewrite は使わない
- OpenSpec: change 作成 → proposal/design/spec/tasks → `openspec validate <id> --strict` → proposal approval → 実装 → `openspec validate --all --strict` → 実装 PR merge → `chore/archive-<id>` で archive + spec sync PR
- 品質ゲート: `cargo test` / `clippy` / `fmt` / `nix flake check` / OpenSpec strict validation
- Windows backend: `windows-2025` non-required laneでnative launcher compile/contract/smokeを確認し、Nix operational behaviorはWSL/Linux execution-sideで保持する
- branch protection required checks の変更は段階移行。新 aggregator を既存 required contexts と並行稼働させてから切り替える
- 手動 CLI 実行時は `XDG_STATE_HOME` を temp に向け、実 state を汚染しない
- 詳細: [CONTRIBUTING.md](../CONTRIBUTING.md) / [RELEASE.md](../RELEASE.md) / [Windows / WSL2 guide](./windows-wsl2.md)
