# Tasks

## 1. 生成 script + workflow

- [x] 1.1 `scripts/ci/generate-release-metadata.sh`: TAG/SHA 引数から `schneeforge-release.json` を生成 (schneeforge.toml の schema/systems を反映)
- [x] 1.2 script 内の自己検証: version==TAG / channel 整合 / configuration_schema==manifest schema / systems 非空
- [x] 1.3 `release.yml`: 生成 step 追加 + CHECKSUMS.txt への sha256 含入 + assets へ追加
- [x] 1.4 `check.yml` release-artifact-check: 同一 script を dummy tag (stable + preview) で実行する step を追加

## 2. core: ReleaseMetadata 型

- [x] 2.1 `crates/core/src/release_metadata.rs`: struct (schema/version/channel/source_revision/minimum_schneeforge_version/configuration_schema/systems) + parse (schema==1 以外は error)
- [x] 2.2 `channel_for_version`: prerelease suffix ありなら preview 無ければ stable
- [x] 2.3 `validate(tag)`: version が tag (v 接頭辞除去) と一致 / channel が version から導出されるものと一致 / systems 非空
- [x] 2.4 `fetch(tag)`: `https://github.com/Lamy210/nix_setting/releases/download/<tag>/schneeforge-release.json` を取得 (download_text) → parse。HTTP error 時は fail-closed な error
- [x] 2.5 unit test: parse / validate (各不正 case) / channel 導出 / URL 形式

## 3. CLI

- [x] 3.1 `schneeforge source metadata <tag>`: fetch 結果を整形表示
- [x] 3.2 integration test: 存在しない tag は fail-closed で error (network 依存は error path のみ)

## 4. test / CI

- [x] 4.1 `cargo test` 全 green (core / cli)
- [x] 4.2 `cargo fmt` / `cargo clippy -D warnings` green
- [x] 4.3 script の local 実行確認 (stable / preview 両 case)
- [x] 4.4 openspec validate green / PR 作成 (base: develop)
