## ADDED Requirements

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
