# Spike Report: NixOS/nix-installer as SchneeForge Managed-Nix Provider

**Date:** 2026-08-14
**Phase:** 3 Spike (pre-ADR)
**Host:** Linux 6.14.0-37-generic x86_64, `/home/ubuntu`, `which nix` → empty (fresh host)
**Binary tested:** `nix-installer-x86_64-linux` v2.35.1 (SHA256 `3b49a0b9…` verified against release `SHA256SUMS`)
**Coverage:** Linux x86_64 のみ実測。macOS aarch64 は未実測 (ADR で Final acceptance 条件として別途要求)。

---

## 1. 結論

**◎ 適する。SchneeForge の Managed-Nix Provider 第一候補として採用を推奨。**

理由: `install / uninstall / plan / repair / self-test / split-receipt` が揃い、receipt (`/nix/receipt.json`) ベースの冪等な revert が可能。pinned binary を GitHub Releases から取得し SHA256SUMS で検証できる。SchneeForge と nix-installer は link しない別プロセスであり、GPL/LGPL FAQ でいう "pipes, sockets and command-line arguments" での通信に該当するため、SchneeForge 側コードにライセンス伝染は起きない (LGPL dynamic-link 例外の話ではなく、そもそも集合物が単一プログラムではない)。binary を DMG 等へ bundle する場合は LGPL-2.1 の再配布条件 (NOTICE 保持・対応 source 提供義務) が発生するため、Phase 1 は online download のみとし、bundle 配布は後続の legal 設計で扱う。唯一の懸念は `plan` サブコマンドが root 昇格を要求する点で、SchneeForge 側で preflight UI を調整する必要がある。

---

## 2. ステータス実測

| 項目 | 実測値 |
|------|--------|
| README 宣言 | 全体 "Beta"。Linux x86_64/aarch64・macOS aarch64・Steam Deck・WSL2・Podman・Docker は Stable |
| 最新 release | `2.35.1` (2026-07-15、prerelease=false) |
| main 最終 commit | 2026-08-13 `b687af91` "Merge #216 bump-nix-version" |
| open issues | 40 / stars 332 / forks 17 |
| 主要 contributors | Hoverbear (417)・Mic92 (215)・cole-h (137)・grahamc (100) — いずれも NixOS コアメンバー |
| License | **LGPL-2.1** (`LICENSE` 参照) |
| 派生元 | `DeterminateSystems/nix-installer` の fork (README冒頭 "A fork of the Determinate Nix Installer")。Determinate 側も 2026-08-10 push で活発 (stars 3671) |

メンテナンスは**極めて活発**。1年前の前回提案時と status は変わらず Beta 宣言維持だが、リリース頻度は月1-2回、PR #216 で Nix バージョン追従中。

---

## 3. CLI surface 実測 (binary `--help` より)

```
nix-installer [OPTIONS] <COMMAND>
Commands:
  install        install [OPTIONS] [PLANNER-SUBCOMMAND] — planner は subcommand (linux/steam-deck/ostree)
                 ※ pre-built plan.json を流す場合は positional で <path> (plan と planner-subcommand は排他) 〔2026-08-14 修正: 当 spike の初出時は --plan long flag と記載したが、2.35.1 の src/cli/subcommand/install/mod.rs では `#[clap(env = "NIX_INSTALLER_PLAN")] pub plan: Option<PathBuf>` のみで long 指定が無く positional であることを実 binary で確認した〕
  repair         repair {hooks|sequoia} — shell profile 修復 / macOS Sequoia _nixbld 回復
  uninstall      uninstall [RECEIPT]  default: /nix/receipt.json
  self-test      Nix が動くか自己診断
  plan           plan [planner-subcommand] — JSON を stdout または --out-file へ
  split-receipt  receipt を phase1 (store 以外) / phase2 (store 掃除) に分割
```

**install 主要 flags (実測):**
- `--enable-flakes`・`--extra-conf <lines>`・`--skip-nix-conf`・`--add-channel`
- `--force`・`--no-confirm`・`--no-modify-profile`・`--explain`
- `--nix-package-url <URL>` (version pin 用、非推奨注意)
- `--proxy`・`--ssl-cert-file`・`--nix-build-group-{name,id}`・`--nix-build-user-{prefix,count,id-base}`
- 全 flags に `NIX_INSTALLER_*` env var が対応 (env-driven automation 向き)

`--no-start-daemon` は top-level `install --help` では見えず、planner-specific option として扱われている (Linux planner の `self.init.start_daemon` で制御)。README には引き続き言及あり。

---

## 4. Receipt / Plan 構造

**Receipt** (`/nix/receipt.json`): `InstallPlan` 構造体 (`src/plan.rs`) を `serde_json::to_string_pretty` でシリアライズした JSON。

```rust
pub struct InstallPlan {
    version: Version,
    actions: Vec<StatefulAction<Box<dyn Action>>>,
    planner: Box<dyn Planner>,
}
```

- `actions` は実行したすべての stateful action (create-user, start-service, provisioning-file 等) の列。`uninstall` はこれを**逆順 revert** する。
- `planner` は linux/macos/steam-deck/ostree のいずれかで、再現に必要な設定を保持。
- 同バイナリのコピーも `/nix/nix-installer` へ保存 (uninstall 時に PATH 不要で呼べる)。

**Plan JSON**: `plan` サブコマンドで事前取得可能 (編集して `install <path>` へ流す。plan は positional)。plan と planner subcommand は排他で、flags (`--no-confirm` 等) は plan 利用時にも有効 (`--enable-flakes` は plan 生成時に plan へ焼き込まれるため replay 時は不要)。〔2026-08-14 修正: 初出時の `--plan` 記載は誤り。実 binary 検証で positional であることを確認〕ただし**root 権限が必要** — planner が `/etc` 等を事前スキャンするため。実測:

```
$ ./nix-installer plan linux --no-modify-profile
INFO nix-installer v2.35.1
`nix-installer` needs to run as `root`, attempting to escalate now via `sudo`...
```

SchneeForge は preflight で plan JSON を取得して差分表示する設計にする場合、**sudo パスワード入力を先に要求**する UX になる。

---

## 5. macOS Apple Silicon 対応

- README で "Stable (see note)" だが、**note が README 本文に存在しない** (ドキュメント不備、要フォロー)。
- 実装上の裏付けは十分: `src/planner/macos/mod.rs` に `Macos { case_sensitive, volume_label, root_disk }` 構造体、APFS Volume 設定をネイティブ対応。
- macOS 固有 defaults: build user prefix `_nixbld` (先頭アンダースコア)、UID base `350`。
- `repair sequoia` サブコマンド実在 (macOS 15 Sequoia が `_nixbld` ユーザーを乗っ取る問題の回復)。
- 既知の落とし穴 (docs/quirks.md): **nix-darwin を残したまま Nix を uninstall すると SSL cert が壊れる**。最新 installer は nix-darwin 残留時に uninstall を拒否する安全機能あり。SchneeForge は uninstall 前に nix-darwin を先に外すシーケンスを保証すること。

---

## 6. Distribution / Supply chain

```
https://github.com/NixOS/nix-installer/releases/download/2.35.1/nix-installer-x86_64-linux
https://github.com/NixOS/nix-installer/releases/download/2.35.1/nix-installer-aarch64-darwin
https://github.com/NixOS/nix-installer/releases/download/2.35.1/SHA256SUMS
```

- tag は **`v` 無し** (`2.35.1` が正しい、`v2.35.1` は 404)。
- asset 4種 + `SHA256SUMS` + `nix-installer.sh` (curl|sh ラッパー)。
- v2.34.5 から **SLSA provenance attestation** (PR #176) と **SHA256SUMS** (PR #177) が添付。sigstore/cosign verify 可能。
- binary は `static-pie linked, stripped` (34MB)。glibc 不要で portable。
- SchneeForge は Tauri に asset を bundle せず、初回起動時に version-pinned download + SHA256 検証が綺麗。

---

## 7. Lix / 公式 shell との比較表

| 軸 | NixOS/nix-installer | lix-installer | 公式 shell (nixos.org/nix/install) |
|----|---------------------|---------------|-------------------------------------|
| 入る Nix | upstream Nix | **Lix** (fork) | upstream Nix |
| コードベース | Determinate fork | Determinate fork の Lix 派生 | shell script (別物) |
| uninstall / revert | `uninstall` + receipt で完全 revert。`split-receipt` で2-phase化 | 同等 (fork 由緒) | **無し** (手動 rm/-userdel) |
| plan JSON | あり (`--out-file`) | あり | 無し |
| macOS aarch64 | Stable・APFS 対応 | Stable | 動くが Sequoia 手対応 |
| nix-darwin 共存 | uninstall 保護あり | **nix-darwin 統合強め** (PR 活発) | 非対応 |
| license | LGPL-2.1 | LGPL-2.1 | MIT (script) |
| checksum / 署名 | SHA256SUMS + SLSA | lix.systems 独自 | **無し** |
| Distribution | GitHub Releases (pinned 可) | `install.lix.systems/lix` | `nixos.org/nix/install` (latest固定) |

**Lix 由来の機能差**: README に明示的な差分記載なし。実質的には "Lix を入れるか upstream Nix を入れるか" の違いのみ。SchneeForge が nix-darwin/Home-Manager を標準とする場合、Lix でも動作するが、コミュニティサイズ・npm cache 互換性・GitHub Actions ecosystem は upstream Nix の方が大きい。

---

## 8. SchneeForge 統合上のリスク

1. **License (LGPL-2.1)**: SchneeForge (Tauri/Rust) と nix-installer は link しない別プロセスであり、GPL/LGPL FAQ でいう "pipes, sockets and command-line arguments" での通信に該当するため、LGPL の dynamic-link 例外の話ではなく、そもそも集合物が単一プログラムとは見なされない。したがって SchneeForge 側コードにはライセンス伝染は起きない。binary を DMG 等へ bundle して再配布する場合は、LGPL-2.1 の再配布条件 (NOTICE 保持・対応 source 提供義務、改変無しすれば v2.1 で止めてよい) を満たす必要があるため、Phase 1 は **online download のみ**とし、bundle 配布は後続の legal/compliance 設計で扱う。
2. **Offline 対応**: pinned binary を事前キャッシュできなければ、SchneeForge 初回起動時に online 必須。Rust core 内の `BootstrapDownloader` (reqwest 等) で取得し、アプリデータ配下にキャッシュすれば、二回目以降は offline で install 可能。**`tauri-plugin-http` は使わない** — 任意 URL 取得権限を Frontend へ渡さず、bootstrap manifest validation を Core に閉じ込めるため。
3. **`plan` が root 必須**: preflight を 2 段階に分ける設計にする。SchneeForge 側で root 不要の概要表示 (/nix, nix-daemon, build users, shell profiles, flakes を変更する旨) を出し、ユーザーが Continue した後に管理者認証 → `nix-installer plan` で詳細 plan を取得する。`--explain` は root 昇格後の人間可読表示として併用。
4. **macOS nix-darwin 順序**: SchneeForge の `doctor` は nix-darwin 検出時に「Nix を消す前に nix-darwin を外す」ガイドを必ず出すこと。本家 installer にも nix-darwin 残留時の uninstall 拒否チェックがあるが、SchneeForge 側でも順序を保証する。
5. **バージョン追従**: installer が Nix バージョンを内部 pin しており `--nix-package-url` 上書きは非推奨。SchneeForge 側で「Nix バージョン指定」を UI に出さず、installer tag で間接指定する設計が安全。
6. **JSON log は stderr**: `--logger json` は stderr へ出力される (`with_writer(std::io::stderr)`)。SchneeForge は stderr を JSON Lines として stream parse する。ただし installer 内部の `Step: CreateUsers` 等のメッセージに深く依存せず、SchneeForge 側の大きな phase (Download / Verify / Waiting for privilege / Planning / Installing / Post-install verification) は自前で管理し、installer 内部のメッセージは詳細 progress として best-effort 表示する程度に留める。

---

## 9. 次のアクション提案

**ADR を書くのに十分な情報は揃った。** 追加調査は不要。ただし macOS aarch64 実測のみ、ADR を Final Accept の前に 1 回だけ実施する。

推奨される Phase 4 ADR の決定事項:

- **Provider**: NixOS/nix-installer (◎)
- **取得方式**: GitHub Releases から version-pinned で `nix-installer-{arch}` を download、SHA256SUMS で検証、アプリデータ配下へキャッシュ (offline 対応)
- **実行方式**: subprocess で `install --no-confirm --logger json --extra-conf ...` を呼び出し、`--logger json` の **stderr** を JSON Lines としてストリーム parse して progress UI に表示。SchneeForge 側で大きな phase を自前管理し、installer 内部メッセージは best-effort 表示。
- **revert**: `/nix/receipt.json` を読み `uninstall --no-confirm` で完全 revert (nix-darwin 残留時は事前警告)。SchneeForge は revert logic を再実装しない。
- **flake**: `--enable-flakes` をデフォルト有効 (SchneeForge は flake前提)
- **Nix version**: installer が選択する version を使用。ユーザー UI から直接指定させない。
- **Supply chain**: SchneeForge 側の version bump CI で upstream release 取得 → `gh attestation verify` → SHA256 確認 → `bootstrap-manifest.toml` へ version + expected SHA256 を commit。runtime は manifest の SHA256 を download binary と local 比較するだけ (gh / cosign は利用者 PC に不要)。
- **Plan UX**: 2 段階 Plan。SchneeForge preflight (権限不要) で概要表示 → Continue → 管理者認証 → `nix-installer plan --out-file ...` → Detailed Plan → Install。
- **macOS**: ADR Status を `Accepted provisionally` とし、**Final acceptance には macOS aarch64 smoke test (install / self-test / flakes / receipt / uninstall / cleanup 確認) を必須条件とする**。

**未解決 (ADR の Open Questions に記載):**
- README の "Stable (see note)" の note 行方が不明。本家 issue で要確認。
- nix-installer と nix-darwin のインストール順序 (Nix 先か nix-darwin 先か) を SchneeForge でどう規定するか。

---

## Sources

- NixOS/nix-installer README: https://github.com/NixOS/nix-installer
- Releases (tag 2.35.1): https://github.com/NixOS/nix-installer/releases/tag/2.35.1
- `src/plan.rs` (InstallPlan 構造): https://github.com/NixOS/nix-installer/blob/main/src/plan.rs
- `src/cli/subcommand/` (CLI 定義): https://github.com/NixOS/nix-installer/tree/main/src/cli/subcommand
- macOS planner: https://github.com/NixOS/nix-installer/blob/main/src/planner/macos/mod.rs
- quirks (nix-darwin 共存): https://github.com/NixOS/nix-installer/blob/main/docs/quirks.md
- DeterminateSystems (fork 元): https://github.com/DeterminateSystems/nix-installer
- Lix installer: https://git.lix.systems/lix-project/lix-installer / https://lix.systems/install/
- 公式 shell installer: https://nixos.org/download/
- SLSA provenance PR #176: https://github.com/NixOS/nix-installer/pull/176
- SHA256SUMS PR #177: https://github.com/NixOS/nix-installer/pull/177
