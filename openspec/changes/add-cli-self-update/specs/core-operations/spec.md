## ADDED Requirements

### Requirement: 本体の自己更新

core SHALL は実行 binary を channel の最新 release へ自己更新できる。
tag 解決は「利用可能 release の解決」と同じ規則 (`git ls-remote --tags`
→ `latest_tag_for_channel`) に従う。binary asset は platform 毎の提供
条件 (darwin は aarch64 のみ / linux は x86_64 のみ) で選択し、
`CHECKSUMS.txt` の sha256 と突合してから置換する。置換は同一
filesystem 上の temp file → rename で atomic に行い、検証失敗時は
実行 binary を一切変更しない (fail-closed)。

#### Scenario: 最新版では no-op

- **WHEN** 実行 version が channel の最新 tag と同等以上
- **THEN** 何も download / 置換せず、最新である旨の結果を返す

#### Scenario: CHECKSUMS 突合による検証

- **WHEN** download した binary の sha256 が `CHECKSUMS.txt` の該当
  asset entry と一致する
- **THEN** 実行 binary を新 binary へ atomic に置換し、移行元 / 移行先
  version と置換 path を結果として返す

#### Scenario: 検証失敗で実行 binary を保護

- **WHEN** sha256 が一致しない、または `CHECKSUMS.txt` に該当 asset の
  entry が存在しない
- **THEN** error を返し、実行 binary は変更しない

#### Scenario: 非対応 platform は download 手前で拒否

- **WHEN** macOS x86_64 または Linux aarch64 で自己更新を実行する
- **THEN** fail-closed に error を返す (install.sh と同一の提供条件)

#### Scenario: 書き込み権限なし

- **WHEN** 実行 binary の置換に必要な directory への書き込み権限が無い
- **THEN** 手動更新 (`sudo` 実行または install.sh) を案内する structured
  error を返す
