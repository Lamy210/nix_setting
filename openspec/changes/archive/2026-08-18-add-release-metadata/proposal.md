# Proposal: Release Metadata 追加 (v2 §27)

## Why

Release asset には現在 binary / DMG / SBOM / CHECKSUMS しか無く、
GUI や CLI は「ある release が何か」(version / channel / 対応 systems /
必要な schneeforge 版数) を release から読み取れない。§28 GUI
Dashboard の「Installed / Available」表示や update の案内は、release
側に machine-readable な metadata があることが前提になる。

§27 設計の metadata JSON を release asset として添付し、生成・検証を
release unit 保証 (1 release = 1 source tree = 1 checksum set) に
組み込む。

## What Changes

- **workflow**: `scripts/ci/generate-release-metadata.sh` (新規,
  python3 で toml 読み取り) が `schneeforge-release.json` を生成:
  schema=1 / version (tag 由来) / channel (prerelease なら preview) /
  source_revision (tag の commit SHA) / minimum_schneeforge_version /
  configuration_schema (schneeforge.toml の schema) / systems
  (schneeforge.toml の enabled systems)。生成後に自己検証
  (version==tag / channel 整合 / systems 非空)
- **release.yml**: metadata を生成し CHECKSUMS.txt と release assets
  へ追加
- **check.yml release-artifact-check**: 同一 script を dummy tag
  (stable / preview 両方) で実行し検証 (同一 script 原則)
- **core**: `release_metadata.rs` — `ReleaseMetadata` の parse /
  検証 (schema / version-tag 整合 / channel 整合 / systems 非空) と
  release asset URL からの fetch
- **CLI**: `schneeforge source metadata <tag>` — release asset を
  fetch して表示

## Impact

- 既存 asset は不変。metadata は追加のみ (1 release = 1 checksum set
  の原則は metadata を CHECKSUMS に含めることで維持)
- 次回 release (rc.6 以降) から asset に含まれる。過去 release には
  遡及しない (fetch は 404/parse error で fail-closed)
- specs: `core-operations` に Release Metadata の parse・検証・fetch
  requirement を ADDED
