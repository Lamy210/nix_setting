# gui-dashboard Specification

## Purpose
SchneeForge Desktop の Dashboard で、現在適用中の version / profile / channel / revision と取得可能な release 情報を安全に可視化し、offline 時も診断可能な状態を保ちながら更新有無と GitHub Releases への導線を提供する。

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

### Requirement: macOS GUI signed self-update

SchneeForge Desktop SHALL provide an in-app update path on supported macOS aarch64 builds using the Tauri updater signature verification model. The updater action MUST NOT be exposed when the build is not configured with an active production updater trust root.

#### Scenario: Signed update is available
- **WHEN** a macOS aarch64 build with updater enabled checks the configured endpoint and a newer signed update is available
- **THEN** Dashboard exposes an app update action with the available version
- **AND** installation occurs only after the user explicitly confirms the update
- **AND** download/install progress is visible

#### Scenario: Update check or install fails
- **WHEN** network, manifest, signature verification, download, or install fails
- **THEN** the currently installed app remains usable and unchanged
- **AND** the GitHub Releases link fallback remains available
- **AND** the error is shown without converting the failure into a successful update state

#### Scenario: Updater is disabled or unsupported
- **WHEN** the app runs on Linux, Windows, or a build without production updater activation
- **THEN** automatic app update controls are not shown
- **AND** the existing release/update guidance remains available

#### Scenario: Update install completes on macOS
- **WHEN** the signed update is installed successfully
- **THEN** the user can restart SchneeForge to run the new version
- **AND** no downgrade is performed automatically

### Requirement: GUI updater IPC contract

Desktop backend SHALL own updater check/install state and expose stable Tauri commands to the frontend. Pending update state MUST NOT be reconstructed from untrusted frontend-supplied URL or signature data.

#### Scenario: Frontend checks for update
- **WHEN** frontend invokes the updater check command
- **THEN** backend returns only serializable update metadata and stores the pending signed update internally

#### Scenario: Frontend installs pending update
- **WHEN** frontend invokes install after confirmation
- **THEN** backend installs only the internally stored pending update
- **AND** frontend does not provide the artifact URL or signature used for verification

### Requirement: GUI updater binds signed artifact version to announced version

Activated macOS GUI updater builds MUST require the updater signature to carry the app version and MUST reject
an update when the signed version differs from the version announced by the updater endpoint.

#### Scenario: Signed version matches announced version
- **WHEN** updater endpoint announces version N and the downloaded artifact signature is bound to version N
- **THEN** signature/version validation may proceed to the existing update installation flow

#### Scenario: Signed version is missing
- **WHEN** updater signature does not contain a signed app version
- **THEN** the update is rejected
- **AND** the existing app remains unchanged
- **AND** the GitHub Releases fallback remains available

#### Scenario: Signed version differs from announced version
- **WHEN** updater endpoint announces version N but the valid artifact signature is bound to version M
- **THEN** the update is rejected as a signed-version mismatch
- **AND** no downgrade or replacement occurs
