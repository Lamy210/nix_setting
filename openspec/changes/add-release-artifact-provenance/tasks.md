# Tasks: add-release-artifact-provenance

- [x] 1. release.yml: release job に job level permissions
      (`contents: write` + `id-token: write`) と attest step
      (`actions/attest-build-provenance` SHA pin) を追加
- [x] 2. RELEASE.md: アセット・ノートに provenance 項目と検証方法を追記
- [x] 3. Final Acceptance 手順書 (gate 0-2) に attestation 検証を追記
- [x] 4. openspec validate で change の構文を確認
