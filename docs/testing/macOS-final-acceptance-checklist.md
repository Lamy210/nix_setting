# macOS Apple Silicon Final Acceptance 手順書

ADR-0001 の Final Acceptance (provisionally accepted → Accepted 昇格) と
PR #11 で未実施だった「Finder からの .app 起動 smoke」を 1 本のフローで通す。

- 対象: Apple Silicon Mac (aarch64)
- 前提: **disposable な環境** (uninstall → reinstall まで含む。重要な環境では実施しない)
- 作業時間の目安: 30〜60 分 (初回の nix download を含む)
- 記録: 各 gate の `[ ]` に結果 (✅ / ❌ + 例外 log) を記入する。
  全 green なら ADR-0001 の Status を `Accepted` へ更新する PR を出す

## フロー全体像

```text
A. Environment    fresh Apple Silicon macOS・Nix なし・state なし
B. Bootstrap      install.sh → pinned CLI → checksum → staging → install
C. Managed Nix    receipt / ownership / daemon / store / flakes
D. CLI            nix doctor / doctor / status / plan
E. Finder         SchneeForge.app を Finder 起動 → minimal GUI PATH で Nix 検出
F. Idempotency    2 回目 install → ExistingNixDetected で安全に拒否
G. Uninstall      ownership 確認 → uninstall → cleanup 確認
H. Reinstall      再 install → 正常導入 (lifecycle 一周)
I. Final          ADR-0001 provisionally accepted → Accepted
```

## 0. 検証対象の pin (最重要)

**「何を検証しているか」を曖昧にしない。** 検証は必ず
「release pipeline が実際に配る artifact」と「同一 source ref」の組で行う。

- current main の installer (one-liner) は **RC.1-era の legacy Nix shell
  installer** であり、RC.2 acceptance の検証対象ではない
- RC.1 の CLI asset は plan `--out-file` P0 (issue #14 で発見、PR #18 で修正) を
  含むため、**RC.1 asset での検証は不可**
- release pipeline は **tag push でのみ発火**する (workflow_dispatch は無い)。
  したがって本手順は以下の順序の「Prerelease assets 生成後」に実施する:

```text
PR (本手順書) merge → release/v0.2.0-rc.2 branch → version bump → release PR
→ main → CI green → merge → v0.2.0-rc.2 tag push → Prerelease + assets 生成
→ 本手順 (Final Acceptance)
   → PASS → ADR-0001 を Accepted へ昇格
   → FAIL → ADR は provisional のまま維持 → fix → RC.3 で出し直し
```

### 0-1. install.sh の取得

install.sh も artifact と同一 ref から取得する (develop HEAD ではない):

```bash
curl -fsSL \
  https://raw.githubusercontent.com/Lamy210/nix_setting/v0.2.0-rc.2/install.sh \
  -o /tmp/install.sh
less /tmp/install.sh   # 内容を確認してから実行
```

### 0-2. CLI binary の保存 (gates D / F / G で使用)

install.sh は CLI を staging から実行した後に **staging を削除**する。
one-liner 完走後は `schneeforge` command がどこにも残らない
(Home Manager profile にも入らない) ため、検証用 CLI は release asset から
事前に保存しておく:

```bash
# 以降の手順は全てこの bash session で続ける (pipefail と $SF を使い回す)
set -o pipefail
ACCEPT_DIR="/tmp/schneeforge-rc2-acceptance"
gh release download v0.2.0-rc.2 -R Lamy210/nix_setting \
  -p 'schneeforge-aarch64-darwin' -D "$ACCEPT_DIR"
SF="$ACCEPT_DIR/schneeforge-aarch64-darwin"
chmod +x "$SF" && "$SF" --version
```

以降の log 取得は `cmd 2>&1 | tee X.log` の直後に `X_rc=$?` で exit code を
変数に残す (`set -o pipefail` が無いと `$?` は tee の exit code になり、
失敗を見逃す)。

- 手元 `cargo build` での検証は unit smoke にはなるが **Release Acceptance
  にはならない** (Linux で「CI binary と release binary が違って壊れた」実績が
  あるため。macOS release binary は `nix build` 産)
- rc.2 公開前に通す必要がある場合は付録 B (手元 build)。ただし D8 の
  /dev/tty 経路など install.sh 由来の検証は含まれない点を記録に明記する

- [ ] gate 0: 検証対象 ref と artifact (tag・SHA) を記録に明記した

## A. Environment (fresh であることの保証)

```bash
# いずれも「無い」こと (Nix が既にいる環境では実施しない)
[ -d /nix ] && echo "NG: /nix exists" || echo "OK"
command -v nix && echo "NG: nix in PATH" || echo "OK"
[ -d "$HOME/.nix-defexpr" ] && echo "warning: legacy nix remnants" || echo "OK"
[ -e "$HOME/.local/state/schneeforge/state.json" ] && echo "warning: SF state" || echo "OK"
```

- [ ] gate A: `/nix` 無し・`nix` command 無し・SchneeForge state 無しを確認

## B. Bootstrap (install.sh 経路)

```bash
bash /tmp/install.sh 2>&1 | tee bootstrap.log
bootstrap_rc=$?
echo "bootstrap_rc=$bootstrap_rc"   # gate B1 は 0 であること
```

確認ポイント:
- CLI binary の download → CHECKSUMS.txt SHA256 検証 → root-owned staging
  (`/private/var/db/schneeforge/bootstrap/`) → root 側再検証 → `schneeforge nix install`
  の順で log が出ること
- D8 最終確認 (detailed plan → y/N) が /dev/tty 経由で表示されること
- repository が `--branch v0.2.0-rc.2 --depth 1` で clone されること
  (detached HEAD。`schneeforge sync` が pinned no-op 案内になることは
  PR #18 で担保済み)
- **install.sh が完走すると、続く `bootstrap.sh` まで自動実行される**
  (macOS では nix-darwin switch → Home Manager apply まで自動適用)。
  つまり gate B 完走時点で環境は full bootstrap 済みであり、
  gate G (uninstall) では nix-darwin を先に外すことが**必須**になる

- [ ] gate B1: install.sh が完走 (`bootstrap_rc=0`)
- [ ] gate B2: checksum verification / staging の log が確認できた
- [ ] gate B3: D8 確認プロンプトが表示され、`y` で先に進んだ

## C. Managed Nix (receipt / ownership / runtime)

```bash
sudo cat /nix/receipt.json | head -40          # receipt
sudo cat /nix/schneeforge-managed.json         # ownership record

# runtime (新しい shell で)
exec $SHELL -l
nix --version
nix store ping
nix flake show github:Lamy210/nix_setting --no-write-lock-file 2>&1 | head
```

- [ ] gate C1: `/nix/receipt.json` が存在し、`version` / `planner.planner` /
      `actions[]` が読める
- [ ] gate C2: `/nix/schneeforge-managed.json` (OwnershipRecord) に
      `installer_version` と `installer_sha256` (64 hex) が入っている
- [ ] gate C3: `nix --version` が応答 (2.35.x 系を想定)
- [ ] gate C4: `nix store ping` が success、flakes が有効
      (`experimental Nix feature` error にならない)

## D. CLI (doctor / status / plan)

one-liner 完走後は `schneeforge` command が PATH に無いため、gate 0-2 で
保存した release binary (`$SF`) を使う:

```bash
cd "$HOME/nix_setting"
"$SF" nix doctor     # receipt / store ping / flakes
"$SF" doctor         # toolchain 診断
"$SF" status         # state
"$SF" plan           # dry-run build
```

- [ ] gate D1: `nix doctor` が receipt / installed: true / store accessible:
      true / flakes available: true
- [ ] gate D2: `doctor` / `status` / `plan` が正常終了

## E. Finder 起動 (PR #11 smoke + fix-path-env-rs 検証)

DMG asset を使う (rc.2 の asset 名は release page で確認):

```bash
gh release download v0.2.0-rc.2 -R Lamy210/nix_setting -p '*.dmg' -D /tmp
hdiutil attach /tmp/SchneeForge_0.2.0-rc.2_aarch64.dmg
# Finder で SchneeForge.app を Applications へ drag & drop
hdiutil detach /Volumes/SchneeForge*
```

**必ず Finder (または `open`) から起動する**。terminal から起動すると
PATH が継承され、minimal GUI PATH の検証にならない。

- [ ] gate E1: Finder 起動で app が開く (crash / blank なし)
- [ ] gate E2: 診断画面で Nix が検出される (shell PATH 非継承でも
      `fix-path-env-rs` + `ToolInventory` で nix path / version が表示)
- [ ] gate E3: Plan / Verify button が応答する (Plan は dry-run build)

## F. Idempotency (2 回目 install の安全な拒否)

```bash
sudo "$SF" nix install 2>&1 | tee install-second.log
second_install_rc=$?
echo "second_install_rc=$second_install_rc"
```

- [ ] gate F1: `ExistingNixDetected` で拒否され、non-zero exit
      (`test "$second_install_rc" -ne 0` が通ること)
- [ ] gate F2: 既存 install (/nix・receipt) が破壊されていない
      (再度 `nix store ping` が通る)

## G. Uninstall / cleanup

**注意**: `/nix` 配下と build users・launchd 設定が削除される。
**gate B が完走している (= bootstrap.sh による nix-darwin 適用済み) 場合は、
uninstall の前に必ず nix-darwin を外す** (SSL cert 破損防止):

```bash
sudo nix --extra-experimental-features "nix-command flakes" \
  run nix-darwin#darwin-uninstaller
```

その後:

```bash
sudo "$SF" nix uninstall 2>&1 | tee uninstall.log
uninstall_rc=$?
echo "uninstall_rc=$uninstall_rc"
```

確認ポイント:
- ownership record の検証 → cached installer binary の SHA256 再検証 →
  upstream uninstaller 実行、の順で log が出ること
- `--force` 無しで実行し、ownership check が通ること

```bash
[ -d /nix ] && echo "NG: /nix remains" || echo "OK: /nix removed"
sudo dscl . -list /Users | grep _nixbld || echo "OK: build users removed"
sudo launchctl print system/nix-daemon 2>&1 | head -1   # not found なら OK
```

- [ ] gate G1: uninstall が完走 (ownership check 通過・`uninstall_rc=0`)
- [ ] gate G2: `/nix` が消え、build users・launchd service も残っていない

## H. Reinstall (lifecycle 一周の証明)

uninstall 後に再び同じ artifact から install できて初めて Managed Nix
lifecycle が一周する。

```bash
bash /tmp/install.sh 2>&1 | tee reinstall.log
reinstall_rc=$?
echo "reinstall_rc=$reinstall_rc"
nix store ping && echo "reinstall OK"
```

- [ ] gate H1: 再 install が完走し (`reinstall_rc=0`)、receipt / ownership が
      再生成される
- [ ] gate H2: `nix store ping` が通る

最終状態: 環境を綺麗に残すならもう一度 G の uninstall を実行して
fresh に戻す (任意)。

## I. 結果の記録と ADR 昇格

記録は `docs/spikes/2026-08-15-macos-managed-nix-final-acceptance/` に残す:

```text
README.md            概要・結果サマリ (下記 template)
environment.txt      macOS version / machine 種別 (sanitize 済み)
bootstrap.log        gate B の log
doctor-before.txt    gate D の nix doctor 出力
finder-smoke.md      gate E の観察 (スクショは repo 外で管理)
idempotency.log      gate F
uninstall.log        gate G
reinstall.log        gate H
```

**sanitize 注意**: username / hostname / serial / private path 等の端末固有
情報を public repo へ commit しない (sed で置換してから追加する)。

README.md の Result template:

```markdown
## Result

Platform: macOS Apple Silicon
Architecture: aarch64
Verified artifact: v0.2.0-rc.2 (tag) / <commit SHA>
Date: 2026-08-XX
Result: PASS

### Gates

- [x] Fresh host had no Nix
- [x] install.sh bootstrap
- [x] checksum verified
- [x] Managed Nix install
- [x] receipt
- [x] ownership
- [x] store ping
- [x] flakes
- [x] Finder launch
- [x] minimal GUI PATH detection
- [x] doctor
- [x] status
- [x] idempotent install rejection
- [x] uninstall
- [x] cleanup
- [x] reinstall
```

全 gate ✅ なら:

1. 記録 directory を PR で develop へ
2. `docs/adr/0001-managed-nix-provider.md` の `Status:` を
   `Accepted provisionally` → `Accepted (2026-08-XX, macOS aarch64 smoke 実施済み)` へ
3. Open Questions #1 (macOS aarch64 smoke) を解決済みにする

どこかで ❌ になった場合は:
- ADR は provisional のまま
- 失敗 gate・log・環境を issue (or PR) に記録し、修正後に再実施
  (rc.2 asset に問題があれば rc.3 で出し直す)

---

## 付録 A: rc.2 未 release 時の扱い

`install.sh` の pin (`v0.2.0-rc.2`) は rc.2 の Release が存在しないと
CHECKSUMS download に失敗する。**rc.2 release assets 公開後に本手順を
実施するのが原則** (gate 0 参照)。rc.1 asset を使った代替は
plan `--out-file` P0 のため存在しない。

## 付録 B: 手元 build の CLI で事前 smoke する場合

```bash
git clone https://github.com/Lamy210/nix_setting.git ~/nix_setting
cd ~/nix_setting && git checkout develop
cargo build --release -p schneeforge
SF="$PWD/target/release/schneeforge"
sudo env NIX_SETTING_DIR="$HOME/nix_setting" "$SF" nix install
```

以降は gate C から同一 (gate 0・B は「手元 build で代替」旨を記録)。
gates D / F / G も同じ `$SF` を使う。D8 確認は terminal TTY で行われ、
install.sh 経路 (checksum / staging / /dev/tty) の検証は含まれない。
**これは Release Acceptance ではない**点を記録に明記する。
