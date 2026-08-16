## ADDED Requirements

### Requirement: DMG bundle に nix-installer を同梱する場合の LGPL-2.1 コンプライアンス

SchneeForge SHALL は LGPL-2.1 の nix-installer binary を DMG 配布物へ同梱する場合、無改変の再配布とし、ライセンス本文の同梱・クレジット表示・対応 source の提供義務を満たす。方針は ADR-0002 に固定する。

#### Scenario: 同梱 binary は無改変であることを CI が検証する

- **WHEN** DMG build が nix-installer binary を bundle resources へ stage する
- **THEN** binary の SHA256 が `bootstrap-manifest.toml` の pin 値 (upstream SHA256SUMS 由来) と一致することを検証する
- **AND** 不一致の場合 build を fail させる

#### Scenario: ライセンス本文とクレジットを同梱する

- **WHEN** nix-installer を含む DMG を配布する
- **THEN** DMG 内に LGPL-2.1 本文と著作権表示を含める
- **AND** 配布物のドキュメント (README または about 表示) に nix-installer (LGPL-2.1) を含むことを明記する

#### Scenario: 対応 source は written offer で提供する

- **WHEN** binary 配布物に対して対応 source の要求を受けた
- **THEN** upstream の該当 version の source (tag の tarball) を提供する
- **AND** release asset に source 参照 (upstream tag への link 集) を添付し、提供手段と期間を RELEASE.md に明記する

#### Scenario: 改変した binary は再配布しない

- **WHEN** upstream の binary に改変 (patch・rebuild・再 pack) を加える必要性が生じた
- **THEN** 改変内容と対象 version を明示した上で、対応する source を自ら提供できる体制でのみ配布する
- **AND** 無改変再配布の CI 検証 (SHA256 一致) を満たさない binary を bundle しない

### Requirement: offline 環境での初回 install

SchneeForge SHALL は bundle 内 (または cache) に検証済み nix-installer binary が存在する場合、network access 無しで install を完結させる。

#### Scenario: bundle 内 binary からの install

- **WHEN** offline 環境で DMG から起動した SchneeForge が install を実行する
- **THEN** bundle 内の nix-installer binary を SHA256 検証の上 cache へ取り込んで install する
- **AND** network access を一切行わない

#### Scenario: bundle / cache いずれにも無い場合は従来通り明示エラー

- **WHEN** offline 環境で bundle にも cache にも binary が無い
- **THEN** 既存の offline 初回起動時の明示的エラー (network 必須) を返す
