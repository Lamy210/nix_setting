# gui-dashboard Specification

## Purpose
TBD - created by archiving change add-gui-dashboard. Update Purpose after archive.
## Requirements
### Requirement: Dashboard 情報の提供

desktop SHALL は async command `get_dashboard` で `DashboardSnapshot`
を返す。network を伴う available release の解決は UI thread を占有
しない。

#### Scenario: snapshot の取得

- **WHEN** frontend が `get_dashboard` を invoke する
- **THEN** installed (version / profile / channel / applied) と available (ReleaseMetadata または理由) と update_available を持つ snapshot が返る

#### Scenario: offline でも Dashboard は表示される

- **WHEN** available 解決が network error で失敗する
- **THEN** command は error を返さず、available が未知であることと理由を snapshot 経由で返す

### Requirement: Dashboard の表示

frontend SHALL は Dashboard に Installed (version / profile /
channel / applied revision) と Available (version / channel /
systems、取得失敗時は理由) を表示する。available が実行 version より
新しい場合は update の案内を表示する。

#### Scenario: update がある場合の表示

- **WHEN** `update_available` が true
- **THEN** Dashboard は最新版がある旨と available version を表示する

#### Scenario: update が無い場合の表示

- **WHEN** `update_available` が false
- **THEN** Dashboard は最新である旨 (または available version が同等) を表示する

#### Scenario: available 取得失敗の表示

- **WHEN** `available` が None で `available_error` に理由がある
- **THEN** Dashboard は available を「取得できません」と理由と共に表示し、installed は通常通り表示する

### Requirement: frontend と backend の契約一致

frontend の snapshot key 参照 SHALL は backend の serialize key と
一致する。desktop の unit test が両者の対応を検証する (serialize key
存在 + frontend 参照の regression test)。

#### Scenario: key 参照の regression 検証

- **WHEN** desktop の test suite が実行される
- **THEN** `DashboardSnapshot` の serialize key と `main.js` の当該 key 参照が検証される

### Requirement: Releases page への誘導

desktop SHALL は Dashboard の update 案内に「GitHub Releases を開く」
button を表示し、押下で available release の page を既定 browser で
開く。URL は core の純関数 (`release_page_url`) が
`<repo_url>/releases/tag/v<version>` として組み立てる (repo_url は
`SCHNEEFORGE_REPO_URL` 上書きに対応)。開く操作は user 権限で実行し、
鍵・release asset・pipeline は変更しない (GUI 自己更新 Step 2 の
前提を作らない)。

#### Scenario: update がある場合に button が表示される

- **WHEN** `update_available` が true かつ `available` が解決されている
- **THEN** Dashboard の update 案内に「GitHub Releases を開く」button が表示される

#### Scenario: update が無い / available 未解決の場合は隠される

- **WHEN** `update_available` が false、または `available` が None
- **THEN** button は表示されない

#### Scenario: button 押下で release page を開く

- **WHEN** ユーザーが button を押す
- **THEN** `open_release` command が available version を受けて実行され、
  `v<version>` tag の release page が既定 browser で開く

#### Scenario: 開けなかった場合は error を表示する

- **WHEN** opener が URL を開けない (関連付け無し等)
- **THEN** CommandOutput の失敗として error が表示され、GUI は稼働を続ける

