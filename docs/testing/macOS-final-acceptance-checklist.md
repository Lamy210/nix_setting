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
A2. Pre-bootstrap GUI   DMG の SchneeForge.app を Nix 導入前に Finder 起動
                         → crash 無し・Nix Missing 表示 (Nix 無し Mac で動くことの保証)
B. Bootstrap      install.sh → pinned CLI → checksum → staging → install
                  ├ rc.5 以前: 完走時は bootstrap.sh まで自動実行
                  │            (clone → nix-darwin switch → Home Manager apply)
                  └ rc.6 以降: clone なし。managed source (tag pinned) → apply
C. Managed Nix    receipt / ownership / daemon / store / flakes
D. CLI            nix doctor / doctor / status / plan
E. Finder         SchneeForge.app を Finder 起動 → minimal GUI PATH で Nix 検出
                  (post-bootstrap。A2 と対で「Nix 無し/有り」両面を保証)
F. Idempotency    2 回目 install → ExistingNixDetected で安全に拒否
G. Uninstall      ownership 確認 → uninstall → cleanup 確認
H. Reinstall      再 install → 正常導入 (lifecycle 一周)
I. Managed source (rc.6 以降のみ) source init/status/update・fail-closed・GUI
J. Final          ADR-0001 provisionally accepted → Accepted
```

## 0. 検証対象の pin (最重要)

**「何を検証しているか」を曖昧にしない。** 検証は必ず
「release pipeline が実際に配る artifact」と「同一 source ref」の組で行う。

- 検証対象 TAG は環境変数で指定する (手順書に RC 番号を直書きしない):

```bash
TAG="${TAG:-v0.2.0-rc.7}"
```

- current main の installer (one-liner) は **RC.1-era の legacy Nix shell
  installer** であり、RC.2 acceptance の検証対象ではない
- RC.1 の CLI asset は plan `--out-file` P0 (issue #14 で発見、PR #18 で修正) を
  含むため、**RC.1 asset での検証は不可**
- **RC.4 の DMG asset は desktop binary が `/nix/store` の libiconv に link した
  まま release されていた** (RC.5 の修正対象。`release-artifact-check` の DMG
  gate 追加により以降は PR 段階で検出される)。RC.4 asset での検証は不可
- **RC.5 と RC.6 で fresh install の経路が変わる** (PR #54-#61, 2026-08-19/20
  merge。rc.5 asset には未同梱):
  - rc.5 以前: install.sh が `--branch "$TAG" --depth 1` で clone し
    bootstrap.sh まで自動実行
  - rc.6 以降: **clone しない**。CLI binary を release から取得し
    `source init --tag` (managed source) → `apply` (flake ref)。
    `$HOME/nix_setting` は作られない
  - 影響する gate (B / D / F / G) には「rc.6 以降」の注記を付けてある。
    gate I (Managed source) は rc.6 以降でのみ実施する。
    また rc.6 から `schneeforge-release.json` asset が同梱される
    (PR #50。gate I-2 の `source metadata` で使用)
- release pipeline は **tag push でのみ発火**する (workflow_dispatch は無い)。
  したがって本手順は以下の順序の「Prerelease assets 生成後」に実施する:

```text
fix merge → release/vX.Y.Z-rc.N branch → version bump → release PR
→ main → CI green → merge → vX.Y.Z-rc.N tag push → Prerelease + assets 生成
→ 本手順 (Final Acceptance)
   → PASS → ADR-0001 を Accepted へ昇格
   → FAIL → ADR は provisional のまま維持 → fix → 次 RC で出し直し
```

### 0-1. install.sh の取得

install.sh も artifact と同一 ref から取得する (develop HEAD ではない):

```bash
curl -fsSL \
  "https://raw.githubusercontent.com/Lamy210/nix_setting/${TAG}/install.sh" \
  -o /tmp/install.sh
less /tmp/install.sh   # 内容を確認してから実行
```

### 0-2. CLI binary の保存 (gates D / F / G で使用)

install.sh は CLI を staging から実行した後に **staging を削除**する。
one-liner 完走後は `schneeforge` command がどこにも残らない
(Home Manager profile にも入らない) ため、検証用 CLI は release asset から
事前に保存しておく:

```bash
# 以降の手順は全てこの bash session で続ける (pipefail と $SF を使い回す)。
# fresh macOS に gh は無い前提で curl で取得し、CHECKSUMS.txt で SHA256 を
# 検証してから使う (D/F/G では root 実行もするため、保存 binary の verify が必須)
set -o pipefail
TAG="${TAG:-v0.2.0-rc.7}"
ASSET="schneeforge-aarch64-darwin"
BASE="https://github.com/Lamy210/nix_setting/releases/download/$TAG"
ACCEPT_DIR="/tmp/schneeforge-acceptance"
mkdir -p "$ACCEPT_DIR"

curl -fsSL "$BASE/$ASSET"       -o "$ACCEPT_DIR/$ASSET"
curl -fsSL "$BASE/CHECKSUMS.txt" -o "$ACCEPT_DIR/CHECKSUMS.txt"

expected="$(sed -n "s|^\([0-9a-f]\{64\}\)  .*/${ASSET}$|\1|p" "$ACCEPT_DIR/CHECKSUMS.txt")"
actual="$(shasum -a 256 "$ACCEPT_DIR/$ASSET" | awk '{print $1}')"
test -n "$expected" && test "$expected" = "$actual" && echo "OK: checksum verified"

SF="$ACCEPT_DIR/$ASSET"
chmod +x "$SF" && "$SF" --version

# provenance 検証 (attest 導入以降の tag のみ。要 gh。fresh macOS に gh が
# 無ければ任意: 実行環境があるときだけ記録する)
# gh attestation verify --repo Lamy210/nix_setting "$ACCEPT_DIR/$ASSET"
```

以降の log 取得は `cmd 2>&1 | tee X.log` の直後に `X_rc=$?` で exit code を
変数に残す (`set -o pipefail` が無いと `$?` は tee の exit code になり、
失敗を見逃す)。

- 手元 `cargo build` での検証は unit smoke にはなるが **Release Acceptance
  にはならない** (Linux で「CI binary と release binary が違って壊れた」実績が
  あるため)
- 対象 TAG 公開前に通す必要がある場合は付録 B (手元 build)。ただし D8 の
  /dev/tty 経路など install.sh 由来の検証は含まれない点を記録に明記する

- [ ] gate 0: 検証対象 ref と artifact (tag・SHA) を記録に明記した
- [ ] gate 0-3 (attest 導入以降の tag): `gh attestation verify` が通る
      (gh の無い環境では skip した旨を記録)

## A. Environment (fresh であることの保証)

```bash
# いずれも「無い」こと (Nix が既にいる環境では実施しない)
[ -d /nix ] && echo "NG: /nix exists" || echo "OK"
command -v nix && echo "NG: nix in PATH" || echo "OK"
[ -d "$HOME/.nix-defexpr" ] && echo "warning: legacy nix remnants" || echo "OK"
[ -e "$HOME/.local/state/schneeforge/state.json" ] && echo "warning: SF state" || echo "OK"
```

- [ ] gate A: `/nix` 無し・`nix` command 無し・SchneeForge state 無しを確認

## A2. Pre-bootstrap GUI smoke (Nix 無し Mac で起動できることの保証)

RC.4 までは Finder 起動の検証が gate B (Nix install) の後のみで、
「Nix が無い Mac でも SchneeForge.app は起動できる」という製品要件が
実機検証されていなかった。RC.4 の DMG は desktop binary が
`/nix/store/.../libiconv.2.dylib` に link しており、この gate で即座に
発覚する類の defect だったため、RC.5 から gate を追加する。

DMG を download して checksum 検証後に mount する (gate E でも使い回す):

```bash
DMG_NAME="SchneeForge_${TAG#v}_aarch64.dmg"
curl -fsSL "$BASE/$DMG_NAME" -o "$ACCEPT_DIR/$DMG_NAME"

expected="$(sed -n "s|^\([0-9a-f]\{64\}\)  .*/${DMG_NAME}$|\1|p" "$ACCEPT_DIR/CHECKSUMS.txt")"
actual="$(shasum -a 256 "$ACCEPT_DIR/$DMG_NAME" | awk '{print $1}')"
test -n "$expected" && test "$expected" = "$actual" && echo "OK: DMG checksum verified"

hdiutil attach "$ACCEPT_DIR/$DMG_NAME"
# Finder で /Volumes/SchneeForge/SchneeForge.app を起動する
# (terminal から起動すると PATH が継承され、minimal GUI PATH の検証にならない)
```

**この時点では Nix は未導入** (gate B の前) であること:

```bash
[ -d /nix ] && echo "NG: gate B already run?" || echo "OK: pre-bootstrap"
```

確認ポイント:
- app が開く (crash / blank なし)。dyld error dialog も出ない
  (RC.4 の `/nix/store` link はここで即 crash したはず)
- wizard (Set up SchneeForge) が表示され、Nix が NG であることが**正しく**表示される
  (rc.3 の field mismatch のように「常に NG」ではなく、実際の状態に追随する)
- rc.6 以降: この時に wizard を step 進行し、source 選択 step で
  **managed source が既定**であることも確認する (gate I8。install まで進めない)
- gate A と同じ shell で確認する (gate B 実行後は再現できない)

```bash
hdiutil detach /Volumes/SchneeForge*
```

- [ ] gate A2-1: Nix 未導入状態で SchneeForge.app が Finder 起動で開く
- [ ] gate A2-2: Nix Missing が正しく表示される (crash・誤表示なし)

## B. Bootstrap (install.sh 経路)

```bash
bash /tmp/install.sh 2>&1 | tee bootstrap.log
bootstrap_rc=$?
echo "bootstrap_rc=$bootstrap_rc"   # gate B1 は 0 であること
```

確認ポイント (rc.5 以前の clone 経路):
- CLI binary の download → CHECKSUMS.txt SHA256 検証 → root-owned staging
  (`/private/var/db/schneeforge/bootstrap/`) → root 側再検証 → `schneeforge nix install`
  の順で log が出ること
- D8 最終確認 (detailed plan → y/N) が /dev/tty 経由で表示されること
- repository が `--branch "$TAG" --depth 1` で clone されること
  (detached HEAD。`schneeforge sync` が pinned no-op 案内になることは
  PR #18 で担保済み)
- **install.sh が完走すると、続く `bootstrap.sh` まで自動実行される**
  (macOS では nix-darwin switch → Home Manager apply まで自動適用)。
  つまり gate B 完走時点で環境は full bootstrap 済みであり、
  gate G (uninstall) では nix-darwin を先に外すことが**必須**になる

確認ポイント (rc.6 以降の managed 経路):
- `[1/4] Fetching schneeforge CLI (release: ...)` が表示され、**clone が行われない**
  こと (`$HOME/nix_setting` が作られない)
- `[4/4] Applying configuration (managed source: <TAG>)` → dotfile backup →
  `Initializing managed source...` → `Applying configuration...` の順で log が出ること
- apply も nix-darwin switch を実行するため、gate G (uninstall) で
  nix-darwin を先に外すことは rc.6 経路でも**必須** (詳細は gate I-1)

- [ ] gate B1: install.sh が完走 (`bootstrap_rc=0`)
- [ ] gate B2: checksum verification / staging の log が確認できた
- [ ] gate B3: D8 確認プロンプトが表示され、`y` で先に進んだ
- [ ] gate B4 (rc.6 以降): clone されず managed source 経路で完走した
      (詳細な確認は gate I-1)

## C. Managed Nix (receipt / ownership / runtime)

```bash
sudo cat /nix/receipt.json | head -40          # receipt
sudo cat /nix/schneeforge-managed.json         # ownership record

# runtime。exec $SHELL -l は使わない (bash session が置換され $SF / pipefail が消える)。
# 現 session のまま Nix profile を source する
if [ -e /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh ]; then
  # shellcheck disable=SC1091
  . /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh
fi
hash -r
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
cd "$HOME/nix_setting"   # rc.6 以降の managed 経路では checkout が無いため cd 不要
                          # (どの directory からでも実行できる)
"$SF" nix doctor     # receipt / store ping / flakes
"$SF" doctor         # toolchain 診断
"$SF" status         # state
"$SF" plan           # dry-run build
```

- [ ] gate D1: `nix doctor` が receipt / installed: true / store accessible:
      true / flakes available: true
- [ ] gate D2: `doctor` / `status` / `plan` が正常終了

## E. Finder 起動 (PR #11 smoke + fix-path-env-rs 検証)

A2 で mount した DMG asset を使う (gate B で Nix 導入済みの状態で再検証する):

```bash
hdiutil attach "$ACCEPT_DIR/$DMG_NAME"
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
# --repo 明示: sudo で root 側 HOME を見て別 repository を探させない
# (install.sh 自身も sudo env NIX_SETTING_DIR=... で渡している)
# rc.6 以降の managed 経路では checkout が存在しないため --repo 指定無しの
# `sudo "$SF" nix install` に読み替える (embedded manifest で動作)
sudo "$SF" --repo "$HOME/nix_setting" nix install 2>&1 | tee install-second.log
second_install_rc=$?
echo "second_install_rc=$second_install_rc"
grep -q 'ExistingNixDetected' install-second.log && echo "OK: rejected as expected"
```

- [ ] gate F1: exit code と error 内容の両方で検証
      (`test "$second_install_rc" -ne 0` かつ
      `grep -q 'ExistingNixDetected' install-second.log` が通ること。
      non-zero だけだと repository not found 等の別失敗も PASS になってしまう)
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
# rc.6 以降の managed 経路では checkout が存在しないため
# `sudo "$SF" nix uninstall` に読み替える (ownership record は /nix 直下)
sudo "$SF" --repo "$HOME/nix_setting" nix uninstall 2>&1 | tee uninstall.log
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

## I. Managed source (rc.6 以降のみ)

PR #54-#61 (managed release source)・PR #50 (release metadata)・PR #56
(GUI profile 切替) が同梱される rc.6 以降の asset で実施する。
rc.5 以前には当該機能が無いため本 section は skip する (skip した旨を記録に残す)。
「fresh install が clone なしで完走する」ことの実機保証が主目的。

### I-1. install.sh の managed 経路 (gate B の詳細確認)

```bash
# working tree-less: checkout が作られていないこと
[ -d "$HOME/nix_setting" ] && echo "NG: checkout exists" || echo "OK: working tree-less"

# state の source が tag pinned の managed source になっていること
# (ref が検証 TAG と一致・managed: true 相当の flake ref)
cat "$HOME/.local/state/schneeforge/state.json"
```

- [ ] gate I1: fresh install が clone せず managed source で完走
      (`$HOME/nix_setting` が存在しない)
- [ ] gate I2: state.json の source が flake ref `github:Lamy210/nix_setting/<TAG>`
      で、ref が検証 TAG に一致

### I-2. CLI での source 確認・更新・fail-closed

```bash
"$SF" source status          # kind: ... (managed) / ref / channel / revision
"$SF" source metadata --tag "$TAG"   # rc.6 以降の asset のみ
"$SF" profile list           # manifest の profiles と現在の選択
"$SF" profile show

# update は state 更新のみ (checkout 操作なし)。
# 検証 TAG が channel 最新なら no-op になる
"$SF" update 2>&1 | tee source-update.log

# flake.lock 更新 (upgrade) は managed source で fail-closed 拒否されること
"$SF" upgrade 2>&1 | tee upgrade-managed.log
upgrade_rc=$?
```

- [ ] gate I3: `source status` が `kind: ... (managed)`・ref・channel を表示
      (revision は metadata asset があれば `verified`)
- [ ] gate I4: `source metadata` が version / channel / source revision を表示
- [ ] gate I5: `update` が checkout 操作なしで完走
      (最新なら `Already on the latest ... release`)
- [ ] gate I6: `upgrade` が拒否される
      (`test "$upgrade_rc" -ne 0` かつ
      `grep -q 'cannot be updated locally' upgrade-managed.log` が通ること)
- [ ] gate I7: `profile list` / `profile show` が manifest の profiles を表示

### I-3. GUI (wizard の source 選択・Dashboard)

wizard の source 選択 step は初回 setup でしか表示されないため、
**gate A2 (pre-bootstrap) の時に wizard を step 進行して確認する**
(source 選択の確認だけで install 実行まで進めない):

- wizard の source 選択 step で **managed source が既定**で選択されていること
  (clone も選択肢として残る)

Dashboard 側は gate E (post-bootstrap) と同じ Finder 起動で確認する:

- profile 切替 (選択 →「適用」/「既定へ」) が反映されること (PR #56)
- **「ソース更新」ボタン**が応答すること (PR #62 merge 済みの tag のみ。
  update = state 更新のみのため昇格 dialog は出ない)
- managed source では**「アップグレード」ボタンが非表示**になること
  (PR #62 merge 済みの tag のみ)

- [ ] gate I8: wizard の source 選択で managed が既定
- [ ] gate I9: Dashboard の profile 切替が反映
- [ ] gate I10 (PR #62 merge 済みの tag):「ソース更新」が応答・
      「アップグレード」が非表示

## J. 結果の記録と ADR 昇格

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
source-update.log    gate I-2 (rc.6 以降)
upgrade-managed.log  gate I-2 (rc.6 以降)
```

rc.5 以前の asset で実施した場合は gate I を skip した旨を記録する
(gate I を含まない全 gate ✅ で ADR-0001 昇格の条件を満たす)。

**sanitize 注意**: username / hostname / serial / private path 等の端末固有
情報を public repo へ commit しない (sed で置換してから追加する)。

README.md の Result template:

```markdown
## Result

Platform: macOS Apple Silicon
Architecture: aarch64
Verified artifact: $TAG (tag) / <commit SHA>
Date: 2026-08-XX
Result: PASS

### Gates

- [x] Fresh host had no Nix
- [x] Pre-bootstrap GUI smoke (Nix 無しで起動・Nix Missing 表示)
- [x] install.sh bootstrap
- [x] checksum verified
- [x] Managed Nix install
- [x] receipt
- [x] ownership
- [x] store ping
- [x] flakes
- [x] Finder launch (post-bootstrap)
- [x] minimal GUI PATH detection
- [x] doctor
- [x] status
- [x] idempotent install rejection
- [x] uninstall
- [x] cleanup
- [x] reinstall
- [x] managed source install (working tree-less)   ← rc.6 以降
- [x] source status / metadata / update            ← rc.6 以降
- [x] upgrade fail-closed rejection                ← rc.6 以降
- [x] GUI managed source wizard / Dashboard        ← rc.6 以降
```

全 gate ✅ なら:

1. 記録 directory を PR で develop へ
2. `docs/adr/0001-managed-nix-provider.md` の `Status:` を
   `Accepted provisionally` → `Accepted (2026-08-XX, macOS aarch64 smoke 実施済み)` へ
3. Open Questions #1 (macOS aarch64 smoke) を解決済みにする

どこかで ❌ になった場合は:
- ADR は provisional のまま
- 失敗 gate・log・環境を issue (or PR) に記録し、修正後に再実施
  (asset に問題があれば次 RC で出し直す)

---

## 付録 A: 検証対象 TAG 未 release 時の扱い

`install.sh` の pin (`$TAG`) は対象 Release が存在しないと
CHECKSUMS download に失敗する。**release assets 公開後に本手順を
実施するのが原則** (gate 0 参照)。

### RC.4 の判定記録 (Final Acceptance FAIL・参考)

RC.4 は release workflow 完走・CHECKSUMS 検証まで PASS したが、
DMG portability preflight で FAIL となり Final Acceptance を実施していない:

```text
Gate 0 (checksum / metadata 二重検証)  PASS
Mach-O compatibility (arm64 / minos)   PASS
CLI portability (/nix/store 無し)      PASS
DMG portability (/nix/store libiconv)  FAIL  ← desktop binary が
  /nix/store/jspv3c5...-libiconv-115.100.1/lib/libiconv.2.dylib に link。
  release-artifact-check が DMG を検査対象外だったため PR 段階で検出されず。
  RC.5 で DMG を host build 化 + mounted-app gate 追加で修正
```

RC.4 asset を使った検証は不可。

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
