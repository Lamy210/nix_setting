# ADR-0001: Managed Nix Provider に NixOS/nix-installer を採用

Date: 2026-08-14
Status: **Accepted provisionally** (Final acceptance には macOS aarch64 smoke test が必須)
Related: Spike Report `docs/spikes/2026-08-14-nix-bootstrap-provider-evaluation/spike-report.md`

## Context

SchneeForge は Nix を再現性エンジンとして利用するが、Nix 自体をユーザーにインストールさせる手段を持たない。初回起動 (First Run Wizard) で `nix` が未検出のとき、ユーザーに「curl|sh せよ」と表示するだけでは、以下の問題が残る。

1. **非可逆**: 公式 `nixos.org/nix/install` (shell script) には uninstall が無く、手動 `rm -rf /nix` を強いる。
2. **監査不能**: 何が変更されたかの receipt が無いため、doctor / rollback / ownership ledger の前提が崩れる。
3. **nix-darwin との順序制御が不能**: macOS で Nix を消す前に nix-darwin を外さないと SSL cert が壊れる問題を SchneeForge 側で防止できない。
4. **flake・プロキシ・認証などのインストール時オプションを統一できない**: env var / config / preflight を一元化する口が必要。

Phase 3 Spike で NixOS/nix-installer (DeterminateSystems fork 由緒) を Linux x86_64 fresh host で実測し、`install / uninstall / plan / repair / self-test / split-receipt` と `/nix/receipt.json` ベースの冪等 revert が揃っていることを確認した。macOS aarch64 は README 上 Stable だが実測が無く、本 ADR は provisional とする。

## Decision

SchneeForge の Managed-Nix の default provider として **NixOS/nix-installer** を採用する。

### Distribution
- version-pinned な online download のみ。**Phase 1 では DMG 等への binary bundle は行わない。**
- `https://github.com/NixOS/nix-installer/releases/download/{tag}/nix-installer-{arch}` から取得。

### Verification
- SchneeForge の version bump CI で upstream release を取得し、`gh attestation verify` (SLSA provenance) と SHA256SUMS を検証した上で、`bootstrap-manifest.toml` に `version` + `expected sha256` を commit する。
- runtime (利用者 PC) では `gh` / `cosign` を要求せず、manifest に埋め込まれた SHA256 を download binary の local SHA256 と比較するだけとする。

### Execution
- SchneeForge と nix-installer は別プロセス。SchneeForge は nix-installer を **external subprocess** として呼び出し、リンクしない。
- Download / Verify / Privilege escalation / Plan / Install / Post-install verification は SchneeForge 側で phase として管理する。

### Logging
- `--logger json` を指定し、installer の **stderr を JSON Lines として best-effort parse** する。
- installer 内部のメッセージ (`Step: CreateUsers` 等) の schema は unstable である前提で、SchneeForge 側の業務 phase に落とし込んで progress UI を駆動する。installer 内部メッセージへの直接依存はしない。

### Flakes
- `plan` 実行時に `--enable-flakes` を指定し、plan へ焼き込む (SchneeForge は flake 前提)。install replay 時には再指定しない (2.35.1 の plan は positional argument で、planner 経由の設定は使われない)。

### Receipt
- upstream の `/nix/receipt.json` を source of truth とする。SchneeForge 側で receipt を再実装・複製しない。

### Uninstall
- upstream の `nix-installer uninstall --no-confirm` を利用する。**SchneeForge は revert logic を再実装しない。**
- SchneeForge 側の手順:
  1. ownership / safety check
  2. nix-darwin 検出時は **nix-darwin を先に外す** (SSL cert 破損防止)
  3. `/nix/nix-installer uninstall --no-confirm` を subprocess 呼び出し

### Nix version
- installer が内部で選択する Nix version を使用する。`--nix-package-url` での上書きは非推奨のため、SchneeForge UI から Nix version を直接指定させず、installer tag で間接指定する。

### Plan UX
- `nix-installer plan` は root を要求するため、2 段階 Plan とする。
  1. SchneeForge preflight (root 不要): `/nix`, `nix-daemon`, build users, shell profiles, flakes を変更することを表示
  2. ユーザーが Continue → 管理者認証 → `nix-installer plan --out-file ...` → Detailed Plan → Install

### License
- nix-installer は LGPL-2.1。SchneeForge は nix-installer を subprocess で呼ぶ**別プロセス**であり、リンクしないため、GPL/LGPL FAQ における "pipes, sockets and command-line arguments" での通信に該当する。LGPL の dynamic-link 例外の話ではなく、そもそも集合物が単一プログラムとは見なされない。したがって SchneeForge 側コードにはライセンス伝染は起きない。
- binary を DMG 等へ bundle して再配布する場合は LGPL-2.1 の再配布条件 (NOTICE 保持・対応 source 提供義務) が別途発生する。Phase 1 は online download のみとし、bundle 配布は後続の legal/compliance 設計 (別 ADR) で扱う。

## Alternatives Considered

### A. DeterminateSystems/nix-installer (fork 元)
- ◎ 活発 (stars 3671, 頻繁な push)。機能差はほぼ無い。
- ✗ Determinate Nix (商用) 周辺の flake registry や commit 形式の独自拡張と親和性が高く、SchneeForge が暗黙にそれらへ引き寄せられるリスク。NixOS org 配下の方がコミュニティ中立性が高く、SchneeForge の対象が (Determinate ユーザーだけでなく) 一般 Nix ユーザー全体である点に合致する。

### B. lix-installer (Lix)
- ◎ fork 元同一のコードベース。nix-darwin 統合の PR が活発。
- ✗ 入る Nix が Lix (fork) となり、npm cache 互換性や GitHub Actions ecosystem の面で upstream Nix より小さい。SchneeForge が nix-darwin / Home-Manager 標準とする場合、コミュニティサイズを取って upstream Nix を選ぶ。

### C. 公式 shell installer (nixos.org/nix/install)
- ◎ 最も標準的。MIT。
- ✗ uninstall / plan / receipt が一切無い。SchneeForge の doctor / rollback / ownership ledger の前提を満たせない。却下。

### D. SchneeForge 自前実装
- ✗ /nix へのファイル配置・build user 作成・launchd/systemd unit 生成などを自前で実装するのは工程が大きく、既存の安定 installer の再発明になる。却下。

## Consequences

### Positive
- `/nix/receipt.json` を source of truth にできるため、SchneeForge の ownership ledger / doctor / rollback が確実な入力を持つ。
- Linux / macOS 両方で同一 CLI を使えるため、Core 側の分岐が最小になる。
- SLSA provenance + SHA256SUMS により supply-chain の信頼性が高い。
- SchneeForge 側で revert を自前実装しなくてよい。

### Negative
- macOS aarch64 の実測が未完了 (Final acceptance 前に smoke test 必須)。
- README の "Stable (see note)" の note が行方不明のため、macOS で Sequoia `_nixbld` 乗っ取り問題以外に文書化されていない注意点がある可能性。
- LGPL-2.1 binary の再配布は今後の DMG bundle 設計で制約になる。
- installer が内部 pin する Nix version に追従するため、SchneeForge が任意の Nix version を選べない。

### Neutral
- installer の tracing JSON schema に強く依存しない progress 抽出レイヤが必要。
- Phase 1 は online 必須。offline 配布は Phase 2 以降。

## Open Questions

1. **macOS aarch64 smoke (Final acceptance の条件)**: disposable environment で以下を検証する。結果次第で本 ADR の Status を `Accepted` に昇格させる。
   - install → self-test → flakes 動作 → receipt 確認 → uninstall → cleanup 確認
2. **README "Stable (see note)" の note 行方**: 本家 issue で確認し、SchneeForge 側の doctor メッセージに必要なら反映する。
3. **nix-installer ↔ nix-darwin 順序**: Nix 先か nix-darwin 先か。SchneeForge の First Run Wizard で規定する。
4. **nix-darwin の安全な取り外し手順**: nix-darwin 公式が `nix-darwin#darwin-uninstaller` を提供している (`sudo nix --extra-experimental-features "nix-command flakes" run nix-darwin#darwin-uninstaller`、install 済みなら `sudo darwin-uninstaller`)。〔2026-08-14 修正: 旧記述「公式 uninstaller が存在しない」は誤りだった〕SchneeForge 側の案内は公式 uninstaller の実行で Phase 1 暫定対応。SchneeForge からの自動呼び出しは別 change で設計する。
5. **DMG bundle 配布**: 別 ADR で LGPL-2.1 再配布条件を満たす方法を決める。
6. **downgrade**: installer が古い版から新しい版への移行はサポートするが、逆方向 (downgrade) の取扱を SchneeForge 側で保証するか。

## Implementation Note

実装は CLI → GUI の順で薄く載せる。

```
1. schneeforge nix install      (CLI)
2. schneeforge nix doctor       (CLI)
3. schneeforge nix uninstall    (CLI)
4. CLI で安定したら Tauri IPC で GUI 接続
5. privileged-gui-operations    (macOS 管理者認証を Managed Nix + nix-darwin Apply に統一)
6. First Run Wizard 統合
```

Managed Nix をいきなり Tauri から実装しない。Core + CLI で `schneeforge nix install` が完璧に動くことを先に保証する。

### Core module 構成 (予定)

```
crates/core/src/managed_nix/
  mod.rs
  provider.rs     // NixOS/nix-installer 固有の振る舞い
  manifest.rs     // bootstrap-manifest.toml の version + sha256
  download.rs     // reqwest ベースの BootstrapDownloader (tauri-plugin-http 不使用)
  verify.rs       // SHA256 / SLSA 検証
  installer.rs    // subprocess 実行・stderr JSON Lines parse
  receipt.rs      // /nix/receipt.json の読み取り専用 view
```
