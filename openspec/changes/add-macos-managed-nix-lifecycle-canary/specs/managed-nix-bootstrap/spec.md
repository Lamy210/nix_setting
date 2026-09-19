## ADDED Requirements

### Requirement: Disposable macOS release lifecycle acceptance helper

SchneeForge SHALL は release artifact の Managed Nix CLI lifecycle を disposable
Apple Silicon macOS environment で再現可能に検証する manual acceptance helper を
提供する。helper は release tag を入力とし、current checkout の build output や
bootstrap manifest を検証対象として使用してはならない (MUST NOT)。

#### Scenario: Fresh Apple Silicon runner で lifecycle を開始する
- **WHEN** operator が release tag を指定して macOS lifecycle acceptance workflow を手動実行する
- **THEN** runner は arm64 であることを確認する
- **AND** `/nix` と `nix` command が存在しない fresh state を確認する
- **AND** fresh precondition を満たさない場合は destructive operation 前に failure になる

#### Scenario: Release binary の integrity を検証する
- **WHEN** helper が指定 tag の `schneeforge-aarch64-darwin` を取得する
- **THEN** 同じ release の `CHECKSUMS.txt` から expected SHA256 を取得する
- **AND** local SHA256 が一致した binary のみを実行する
- **AND** command は checkout 外で実行し release binary の embedded manifest を使用する

#### Scenario: Managed Nix lifecycle を一周する
- **WHEN** verified release binary で acceptance lifecycle を実行する
- **THEN** `nix install --yes` が成功する
- **AND** receipt / ownership record / store ping / flakes / `nix doctor` を検証する
- **AND** 2 回目 install は `ExistingNixDetected` を含む non-zero exit で拒否される
- **AND** uninstall 後に `/nix` が消える
- **AND** reinstall が成功する
- **AND** final uninstall 後に `/nix` が消える

#### Scenario: GUI Final Acceptance を置き換えない
- **WHEN** automated lifecycle workflow が success になる
- **THEN** Finder GUI smoke / display verification / install.sh interactive D8 gate は未検証のままとする
- **AND** workflow success だけで ADR-0001 の Status を `Accepted` へ変更しない
