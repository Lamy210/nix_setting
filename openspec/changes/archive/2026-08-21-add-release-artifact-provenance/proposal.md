# Proposal: Release artifact への build provenance attestation (Phase E)

## Why

Phase E の supply chain 要件は「GitHub Release + checksum + SBOM + provenance」。
checksum (CHECKSUMS.txt) と SBOM (sbomnix / CycloneDX) は整備済みだが、
**自 release asset の provenance attestation だけ未整備**:

- SchneeForge は upstream (NixOS/nix-installer) の検証に `gh attestation
  verify` (SLSA provenance) を使っている (upstream-nix-installer.yml の
  supply chain 2 層検証)。しかし自 asset を attest していないため、user は
  SchneeForge 自身の asset を同じ手段で検証できない (非対称)
- Final Acceptance 手順書 (gate 0-2) は checksum の SHA256 検証までで、
  「asset がこの repo の CI で build されたこと」の検証が無い

## What Changes

- **release.yml**: release job で asset 一式 (CLI binaries / DMG / SBOM /
  `schneeforge-release.json` / CHECKSUMS.txt) に SLSA build provenance を
  attest する step を追加 (`actions/attest-build-provenance` v4.2.2 を
  commit SHA pin)。attestation 用に release job へ `id-token: write`
  (OIDC) を付与
- **RELEASE.md**: リリース前 checklist に provenance 項目と検証方法
  (`gh attestation verify --repo Lamy210/nix_setting <file>`) を記載
- **Final Acceptance 手順書**: gate 0-2 に attestation 検証手順を追加
  (本変更を含む最初の release 以降の tag で有効)

## 非対象

- 過去 release への遡及 attest。attestation も checksum と同じく
  「1 release = 1 source tree = 1 checksum set」の release 単位で生成する
  (既存 asset の再生成・差し替えは原則違反)
- SBOM の内容拡充や signing / notarization (別要件。macOS notarization は
  既存の release 方針を維持)
