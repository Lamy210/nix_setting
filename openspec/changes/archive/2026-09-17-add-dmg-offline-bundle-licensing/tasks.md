# Tasks

## 1. ADR-0002 の起票

- [x] 1.1 `docs/adr/0002-dmg-bundle-lgpl-redistribution.md` を作成: LGPL-2.1 再配布条件の分析 (本文保持 / source 提供義務 / 改変明示) と SchneeForge の対応方針を決定
- [x] 1.2 Status を `Accepted provisionally` とし、弁護士確認を Open Question に残す
- [x] 1.3 `docs/adr/README.md` の一覧へ追記
- [x] 1.4 ADR-0001 の Open Question 5 に ADR-0002 への解決参照を追記

## 2. 上流の事実確認

- [x] 2.1 nix-installer の license が LGPL-2.1 であることを GitHub API で確認
- [x] 2.2 release に source tarball / zipball が常に存在することを確認 (auto-generated)
- [x] 2.3 release assets (binary 3 arch + SHA256SUMS) の構成を確認

## 3. openspec

- [x] 3.1 `openspec/changes/add-dmg-offline-bundle-licensing/` に proposal / specs / tasks を作成
- [x] 3.2 `openspec validate add-dmg-offline-bundle-licensing --strict` が通ること

## 4. 後続 change での実装 (本 change の対象外)

- [ ] 4.1 DMG resources への binary + LICENSE 同梱 (build script 拡張)
- [ ] 4.2 bundle → cache → download の fallback 順を持つ install 経路
- [ ] 4.3 bundle 内 binary の SHA256 gate を release-artifact-check へ追加
- [ ] 4.4 offline install の E2E test
