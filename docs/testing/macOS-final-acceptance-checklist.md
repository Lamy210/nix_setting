# macOS Apple Silicon Final Acceptance 手順書

ADR-0001 の Final Acceptance (provisionally accepted → Accepted 昇格) と
PR #11 で未実施だった「Finder からの .app 起動 smoke」を 1 本のフローで通す。

- 対象: Apple Silicon Mac (aarch64)
- 前提: **disposable な環境** (実機でも良いが、後述の uninstall で全て戻ることを
  先に理解した上で実施すること。重要な環境では実施しない)
- 作業時間の目安: 30〜60 分 (初回の nix download を含む)
- 記録: 各 checkpoint の `[ ]` に結果 (✅ / ❌ + 例外 log) を記入する。
  全 green なら ADR-0001 の Status を `Accepted` へ更新する PR を出す

## 0. 事前確認 (fresh であることの保証)

```bash
# いずれも「無い」こと (Nix が既にいる環境では実施しない)
[ -d /nix ] && echo "NG: /nix exists" || echo "OK"
command -v nix && echo "NG: nix in PATH" || echo "OK"
[ -d "$HOME/.nix-defexpr" ] && echo "warning: legacy nix remnants" || echo "OK"
```

- [ ] checkpoint 0: `/nix` 無し・`nix` command 無しを確認

## 1. one-line bootstrap (install.sh 経路)

`install.sh` は `SCHNEEFORGE_BOOTSTRAP_VERSION` (現行 pin: `v0.2.0-rc.2`) の
Release から CLI binary を download する。**rc.2 がまだ release されていない場合**
は、この手順書の「付録 A: rc.2 未 release 時の代替」を使う。

```bash
curl -fsSL https://raw.githubusercontent.com/Lamy210/nix_setting/develop/install.sh -o /tmp/install.sh
less /tmp/install.sh   # 内容を確認してから実行
bash /tmp/install.sh
```

確認ポイント:
- CLI binary の download → CHECKSUMS.txt SHA256 検証 → root-owned staging
  (`/var/db/schneeforge/bootstrap/`) → root 側再検証 → `schneeforge nix install`
  の順で log が出ること
- D8 最終確認 (detailed plan → y/N) が /dev/tty 経由で表示されること。
  curl|bash ではなく file 実行でも同じ code path
- repository が `--branch v0.2.0-rc.2 --depth 1` で clone されること
  (detached HEAD になる。`schneeforge sync` が pinned no-op 案内になることは
  PR #18 で担保済み)

- [ ] checkpoint 1a: install.sh が完走 (exit 0)
- [ ] checkpoint 1b: D8 確認プロンプトが表示され、`y` で先に進んだ

## 2. receipt / ownership record の確認

```bash
sudo cat /nix/receipt.json | head -40
sudo cat /nix/schneeforge-managed.json
```

- [ ] checkpoint 2a: `/nix/receipt.json` が存在し、`version` / `planner.planner` /
      `actions[]` が読める
- [ ] checkpoint 2b: `/nix/schneeforge-managed.json` (OwnershipRecord) が存在し、
      `installer_version` と `installer_sha256` (64 hex) が入っている

## 3. Nix runtime の確認 (self-test / flakes / store)

```bash
# 新しい shell を開く (profile の再読み込み)
exec $SHELL -l
nix --version
nix store ping
nix flake show github:Lamy210/nix_setting --no-write-lock-file 2>&1 | head
```

- [ ] checkpoint 3a: `nix --version` が応答 (2.35.x 系を想定)
- [ ] checkpoint 3b: `nix store ping` が success
- [ ] checkpoint 3c: flakes が有効 (`nix flake show` が `error: experimental
      Nix feature` にならない)

## 4. schneeforge doctor / status

```bash
cd "$HOME/nix_setting"
./target/release/schneeforge doctor    # build していなければ cargo build --release -p schneeforge
schneeforge nix doctor                 # PATH に入っていれば
```

- [ ] checkpoint 4a: `schneeforge nix doctor` で receipt / installed: true /
      store accessible: true / flakes available: true が表示される

## 5. GUI: Finder から SchneeForge.app を起動 (PR #11 smoke)

DMG を使う場合 (rc.1 の asset がある):

```bash
gh release download v0.2.0-rc.1 -R Lamy210/nix_setting -p '*.dmg' -D /tmp
hdiutil attach /tmp/SchneeForge_0.2.0-rc.1_aarch64.dmg
# Finder で SchneeForge.app を Applications へ drag & drop
hdiutil detach /Volumes/SchneeForge*
```

または手元で build:

```bash
cd ~/nix_setting/apps/desktop/src-tauri
nix develop --command cargo tauri build
# target/release/bundle/macos/SchneeForge.app を Finder で開く
```

**必ず Finder (または `open`) から起動する**。terminal から起動すると
PATH が継承され、minimal GUI PATH の検証にならない。

確認ポイント (ADR-0001 が前提とする `fix-path-env-rs` の効果):
- 診断 (Diagnostics) 画面で Nix が「見つからない」にならないこと
- Status / Toolchain 表示に nix の path と version が出ること
- First Run Wizard が repository 検出をする場合、NeedsSetup ではなく
  Ready 状態に到達すること (config が適用済みなら)

- [ ] checkpoint 5a: Finder 起動で app が開く (crash / blank なし)
- [ ] checkpoint 5b: 診断画面で Nix detected (minimal GUI PATH でも検出)
- [ ] checkpoint 5c: Plan / Verify button が応答する (Plan は dry-run build)

## 6. dotfiles 適用 (任意・推奨)

```bash
cd "$HOME/nix_setting"
./bootstrap.sh
```

- [ ] checkpoint 6 (任意): apply → verify が完走。nix-darwin switch が
      管理者権限を要求し、明示昇格の確認が表示されること

## 7. uninstall / cleanup

**注意**: `/nix` 配下と build users・launchd 設定が削除される。
disposable 環境で実施すること。nix-darwin を適用した (checkpoint 6 を
実施した) 場合は必ず先に nix-darwin を外す:

```bash
sudo nix --extra-experimental-features "nix-command flakes" run nix-darwin#darwin-uninstaller
```

その後:

```bash
sudo schneeforge nix uninstall
```

確認ポイント:
- ownership record の検証 → cached installer binary の SHA256 再検証 →
  upstream uninstaller 実行、の順で log が出ること
- `--force` 無しで実行し、ownership check が通ること

```bash
# cleanup 確認
[ -d /nix ] && echo "NG: /nix remains" || echo "OK: /nix removed"
[ -f /nix/schneeforge-managed.json ] && echo "NG" || echo "OK"
sudo dscl . -list /Users | grep _nixbld || echo "OK: build users removed"
sudo launchctl print system/nix-daemon 2>&1 | head -1   # not found なら OK
```

- [ ] checkpoint 7a: `schneeforge nix uninstall` が完走
- [ ] checkpoint 7b: `/nix` が消え、build users・launchd service も残っていない

## 8. 結果の記録と ADR 昇格

全 checkpoint が ✅ なら:

1. この file に実施日・環境 (macOS version / machine)・結果を記録した copy を
   `docs/testing/` に残す (または PR description に貼る)
2. `docs/adr/0001-managed-nix-provider.md` の `Status:` を
   `Accepted provisionally` → `Accepted (2026-08-XX, macOS aarch64 smoke 実施済み)` へ
3. Open Questions #1 (macOS aarch64 smoke) を解決済みにする
4. PR → develop merge

どこかで ❌ になった場合は:
- ADR は provisional のまま
- 失敗 checkpoint・log・環境を issue (or PR) に記録し、修正後に再実施

---

## 付録 A: rc.2 未 release 時の代替手順

`install.sh` の pin (`v0.2.0-rc.2`) は rc.2 の Release が存在しないと
CHECKSUMS download に失敗する。rc.2 release 前に smoke する場合は
**rc.1 の binary + develop HEAD の install.sh** で、CLI 側に rc.1→rc.2 間の
破壊的変更が無いことを確認した上で:

```bash
SCHNEEFORGE_VERSION=v0.2.0-rc.1 \
SCHNEEFORGE_REF=v0.2.0-rc.1 \
  bash /tmp/install.sh
```

注意:
- rc.1 の CLI には PR #18 の fix (plan stdout 対応・TOCTOU hardening・
  ref pin・sync 安全化) が**入っていない**。rc.1 binary での install は
  `nix-installer plan --out-file` bug により失敗する可能性が高い
  (issue #14 で発見された P0)。**原則、rc.2 release 後に本手順を実施する**
- どうしても rc.2 前に通す場合は、付録 B (手元 build) を使う

## 付録 B: 手元 build の CLI で通す場合

```bash
git clone https://github.com/Lamy210/nix_setting.git ~/nix_setting
cd ~/nix_setting && git checkout develop
cargo build --release -p schneeforge
sudo env NIX_SETTING_DIR="$HOME/nix_setting" \
  ./target/release/schneeforge nix install
```

以降は手順 2 から同一 (install.sh 経路の checkpoint 1a/1b は
「手元 build で代替」旨を記録する)。この場合 D8 確認は terminal の
TTY で行われる (GUI 経由の /dev/tty 検証は含まれない点に注意)。
