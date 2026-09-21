# Change: harden GUI updater with signed-version binding

## Why

SchneeForge Desktop の GUI updater は Tauri updater の mandatory signature verification を利用するが、
current release build は Tauri CLI 2.11.4 を pin しており、runtime config は
`requireSignedVersion` を有効化していない。

Tauri 2.11.5 では updater signature の trusted comment に app version を含める
security fix が導入された。updater endpoint response 自体は署名されず、従来は
古い正規署名 artifact の URL/signature と、より新しい announced version を組み合わせる
response を構成できた。artifact signature 自体は正しいため、signed-version binding が無いと
version と artifact の対応関係を暗号学的に固定できない。

SchneeForge は production activation 前なので、shipping trust root を provision する前に
version binding を mandatory にしておく。

## What Changes

- release macOS bundle に使う pinned Tauri CLI を 2.11.5 へ更新
- updater plugin config で `requireSignedVersion: true` を常時指定
- runtime public key / endpoint の staged activation model は維持
- ordinary PR/develop build は updater-disabled のまま
- CI contract で CLI pin / digest / signed-version requirement を固定
- release/update docs と STATUS に security requirement を追記

## Security invariant

Activated updater build MUST reject:
- signature に signed version が無い artifact
- endpoint が announce する version と signature trusted comment の version が一致しない artifact

本 change は production key provision / updater activation / real N→N+1 E2E を行わない。

## Impact

- specs: `gui-dashboard`, `release-supply-chain`
- desktop config: updater plugin config
- release build: Tauri CLI pin
- test: static CI contract + desktop config regression
- risk: low; updater is still production-disabled, and this only strengthens future activation
