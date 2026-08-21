# Design: add-release-artifact-provenance

## D1. attest の実行場所と対象

release job で実行する。時点は CHECKSUMS.txt 生成後・`create release` の前。
**attest に失敗したら release は作らない** (fail-closed。checksums 生成後に
置くことで、その release の asset 全てが subject 化できる)。

subject は multiline path 指定 (`subject-path` は glob / 複数 path 対応):

```yaml
subject-path: |
  dist/*/*
  sbom.cdx.json
  schneeforge-release.json
  CHECKSUMS.txt
```

- `dist/*/*`: CLI binaries (`schneeforge-aarch64-darwin` /
  `schneeforge-x86_64-linux`) + DMG (download-artifact が
  `dist/<artifact-name>/` へ展開するため 1 つの glob で両方をカバー)
- CHECKSUMS.txt も subject に含める。「checksum 清单自体の由来」も保証する

## D2. permission

workflow level の `permissions: contents: write` は既存のまま、**release job
のみ** job level で `permissions` を上書きする (build job に OIDC 権限を
渡さない):

```yaml
permissions:
  contents: write   # release 作成 (既存)
  id-token: write   # artifact attestation (OIDC)
```

## D3. action の pin

`actions/attest-build-provenance@4d101475d8b20a2381f78447822ac1eab6504dd8`
(v4.2.2 の commit SHA。repo の全 action と同じ SHA pin 運用)。

## D4. 検証方法

attestation は GitHub の attestations API に保存され (release asset として
は同梱されない)、tag digest + build workflow run に紐付く。検証は upstream
nix-installer と対称の操作:

```bash
gh attestation verify --repo Lamy210/nix_setting <file>
```

Final Acceptance gate 0-2 では、保存済み CLI binary と DMG に対してこれを
実行する (要 `gh`。fresh macOS 想定の手順では任意 item とし、実行環境が
あれば記録する)。

## D5. 遡及適用の禁止

本変更を含む最初の tag 以降のみ attest される。過去 release の asset を
後から attest しない (asset 再生成 = 差し替えは「1 release = 1 source
tree = 1 checksum set」違反のため)。

## D6. test 方針

- workflow 構文は CI の lint job (actionlint) で検証
- attestation の実生成は次回 release の workflow run で確認
  (workflow は tag push でのみ発火するため PR CI では生成されない)
- RELEASE.md / Final Acceptance 手順書の記載と実挙動の一致は
  次回 release 時の checklist 実行で確認
