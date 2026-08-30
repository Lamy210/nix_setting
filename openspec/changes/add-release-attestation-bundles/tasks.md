# Tasks: add-release-attestation-bundles

## 1. 生成 script

- [x] 1.1 `scripts/ci/slsa_predicate.py` — SLSA v0.2 predicate 生成 (builder id / materials sha1 / metadata)
- [x] 1.2 `scripts/ci/attest-release-assets.sh` — artifact ごとに cosign sign-blob / attest-blob / syft SPDX を生成し、identity pin 自己検証をかける
- [x] 1.3 shellcheck 対象 list (check.yml lint job) に script を追加

## 2. workflow / docs

- [x] 2.1 release.yml release job に生成 step を追加し、release files に `attestations/*` を含める
- [x] 2.2 RELEASE.md checklist に attestation bundle 項目 (生成・自己検証・検証方法) を追加

## 3. test

- [x] 3.1 `tests/ci-scripts.bats` に predicate 生成の dummy 検証 (JSON 構造 / builder / materials) を追加

## 4. 品質 gate

- [x] 4.1 `openspec validate add-release-attestation-bundles --strict` 通過
  - note: `openspec validate --all` の gui-operations 失敗は develop 既存 (requirements.15 SHALL/MUST 欠落)。本 change の対象外
- [x] 4.2 shellcheck / actionlint / bats が local (docker) で通る
  - shellcheck: attest-release-assets.sh + ci-scripts.bats / actionlint: 全 workflow / bats: 16 test (新規 2 件含む) green / shfmt 適用済み
