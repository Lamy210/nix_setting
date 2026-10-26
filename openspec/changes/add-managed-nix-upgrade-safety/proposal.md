# Change: Managed Nix の既存ホスト更新を安全に設計する

## Why

Issue #256: `bootstrap-manifest.toml` の nix-installer bump（PR #244）は新規インストールにだけ適用され、既存ホストの `nix` CLI / daemon は更新しない。2026-10 の Nix セキュリティ修正を利用中ホストへ届けるには、**既存インストールの所有者を特定してから、適切な更新主体へ委譲する**必要がある。

`nix-installer` は `sudo -i nix upgrade-nix` を案内する一方、SchneeForge の macOS configuration は `nix.enable = true` の nix-darwin を宣言している。Nix を宣言的に管理する環境に手動で profile 更新を適用すると、次回の switch と競合し得る。CLI の Nix version が上がっていても daemon が旧版のままという危険もある。

## What Changes

- **Phase A / 初回実装:** read-only の `schneeforge nix upgrade inspect [--json]` を追加し、既存の `NixStatus` / `nix doctor` の上に Nix CLI version、daemon reachability、確認可能な daemon version、インストーラー ownership / receipt、更新主体の判定根拠、対応可否（`manual / unknown / n/a`）を構造化して表示する。**実機の状態は変更しない。**
- Unknown を安全な分類として保持し、「installer version == Nix runtime version」「Nix CLI version == daemon version」「repo の設定が現在適用されている」などの根拠のない同一視を禁止する。
- コマンド実行は Core に置き、CLI は表示用 adapter とする。OS native の tool path / `ToolInventory` / existing `Receipt` / `OwnershipRecord` を再利用する。
- `nix-darwin`、NixOS、Lix、Determinate / non-SchneeForge、receipt 破損、daemon version 不明の各環境では、実行可能な自動更新を許可しない（Phase A では全環境 read-only）。
- **Phase B / 別の承認付き実装:** 製品が所有し検証済みの topology に限定して pinned target + provenance / store signature / privilege + lock + crash recovery + rollback check を満たす opt-in 更新を追加する。Phase A と Phase B の承認・受入は分ける。
- 既存の `nix install` / `repair` / `uninstall`、他の `schneeforge upgrade` 等の責務は変更しない。

## Scope / Out of Scope

- 本 change の **実装対象は read-only inventory / inspect のみ**。更新処理 `nix upgrade-nix` 自体、daemon restart、profile switch、`nix-darwin switch`、`/nix` の削除や再インストールは実装しない。
- GUI は後続 change。外部 OS の構成、既存 Nix daemon のセキュリティ状態やユーザーの端末のバージョンは本調査では unknown。
- installer provider 2.35.3 と Nix daemon の fixed version 2.35.3 は同一であると推定しない。
- CI / release artifact の required gates は弱めない。

## Impact

- Specs: `managed-nix-bootstrap`（ADDED Requirements のみ）。
- Planned Phase A code: `crates/core/src/managed_nix/upgrade_inspect.rs`、`crates/core/src/managed_nix/mod.rs`、`crates/core/src/lib.rs`、`crates/cli/src/nix_cmd.rs`、関連 tests。
- Documents: `docs/STATUS.md`、新 CLI 利用ガイド、Issue #256。
- CI: Rust / Nix / OpenSpec / Linux / macOS gates。既存 release 動作と binary command を壊さない。
- Security: 新しい root 権限・ネットワーク・実行可能な updater は本フェーズでは追加しない。

## Approval gate

この PR は **OpenSpec 設計とレビュー用**。厳密な仕様検証・proposal 承認の後に Phase A の実装 PR を作り、完了後に別 PR で OpenSpec archive/spec sync する。Phase B は Phase A の受入後に別 change として設計・承認する。

## Primary references (2026-10-27 verification)

- NixOS/nix-installer README (Upgrading Nix): https://github.com/NixOS/nix-installer/blob/main/README.md#upgrading-nix
- Nix 2.35.2 manual (`nix store info`; availability only): https://nix.dev/manual/nix/2.35/command-ref/new-cli/nix3-store-info.html
- Nix reference (`nix upgrade-nix` experimental; stable-target caveats): https://nix.dev/manual/nix/2.28/command-ref/new-cli/nix3-upgrade-nix
- October 2026 advisory (Nix/Lix fixed lines differ): https://discourse.nixos.org/t/security-advisory-for-nix-and-lix-october-2026/80547
- Repository: `nix-darwin/config/nix-config.nix` (`nix.enable = true`), `docs/adr/0001-managed-nix-provider.md`, `crates/core/src/managed_nix/{receipt,ownership,status}.rs`.
