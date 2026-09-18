## ADDED Requirements

### Requirement: macOS destructive acceptance workflow isolation

Managed Nix install/uninstall を行う macOS lifecycle acceptance workflow は
manual / non-required verification として required CI・release path から分離しなければ
ならない (MUST)。fresh-host contract を壊す Nix setup action を acceptance job の
前段で実行してはならない (MUST NOT)。

#### Scenario: Pull request を作成する
- **WHEN** normal PR CI が起動する
- **THEN** destructive macOS lifecycle acceptance workflow は自動実行されない
- **AND** current required contexts は acceptance workflow の結果に依存しない

#### Scenario: develop へ push する
- **WHEN** topic PR が `develop` へ merge される
- **THEN** destructive macOS lifecycle acceptance workflow は自動実行されない
- **AND** operator が `workflow_dispatch` で明示起動した場合のみ lifecycle を開始する

#### Scenario: Fresh-host acceptance job を準備する
- **WHEN** lifecycle job が開始する
- **THEN** job は `cachix/install-nix-action` 等で Nix を事前導入しない
- **AND** script 自身が fresh Nix state を fail-closed 検証する

### Requirement: macOS acceptance workflow contract regression protection

CI SHALL は acceptance workflow の isolation と release-artifact lifecycle marker を
static contract test で検証する。

#### Scenario: Acceptance workflow が required path へ混入する
- **WHEN** workflow に PR/push trigger、required aggregator dependency、または事前 Nix setup が追加される
- **THEN** repository の workflow contract test は failure になる

#### Scenario: Lifecycle coverage が削除される
- **WHEN** checksum verification、install、ExistingNixDetected、uninstall、reinstall のいずれかの marker が script から失われる
- **THEN** repository の workflow contract test は failure になる
