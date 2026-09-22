# Change: DMG offline bundle 配布と LGPL-2.1 再配布条件 (ADR-0002)

## Why

Managed Nix の install は Phase 1 では online download のみ
(`bootstrap-manifest.toml` で pin した version + SHA256 を検証)。
offline 環境 (閉域網・機密区域) では初回 install が不可能。

offline 対応の自然な形は DMG へ nix-installer binary を bundle する
ことだが、nix-installer は **LGPL-2.1** であり binary の再配布には
LGPL-2.1 の再配布条件が発生する (ADR-0001 OQ5 の宿題。issue #17):

1. LGPL-2.1 本文と著作権表示の保持
2. 対応 source の提供義務 (written offer または同梱)
3. 改変の有無・改変部分の明示

SchneeForge 側 code は subprocess 実行 (集合物として単一プログラム
ではない) のため伝染しない (ADR-0001 判定通り)。問題は**再配布**だけ。

上流の事実確認 (2026-08-16 実測):

- license: LGPL-2.1 (GitHub API `license.spdx_id` 確認済み)
- release assets: binary 3 arch + `nix-installer.sh` + `SHA256SUMS`
- source: release の `zipball_url` / `tarball_url` (auto-generated)
  が常に利用可能。tag `2.35.1` と commit が 1:1

## What Changes

- **ADDED: ADR-0002** — DMG bundle 配布における LGPL-2.1 対応の方針。
  本 change の decision を ADR として固定する
- **ADDED: bundle 形式 (ADR の Decision の実装側要約)**
  - DMG 内 `Contents/Resources/licenses/nix-installer/LICENSE`
    に LGPL-2.1 本文 + NOTICE を同梱
  - DMG 配布物の README / GUI の about 表示に「nix-installer
    (LGPL-2.1) を含む」と明記
  - 対応 source は **written offer 形式**: release asset として
    upstream source tarball への link 集 (`SOURCES.md`) を添付し、
    要求があれば 3 年間提供する運用を RELEASE.md に明記
  - binary は**無改変**で再配布する (SHA256 が upstream SHA256SUMS
    と一致することを CI で検証済みのもののみ bundle)
- **ADDED: offline install path の検証**
  - bundle 内 binary を cache へ取り込んで install する経路の設計
  - network 不要で cache から install する既存動作の test 拡充
- **法務確認 flag**: ADR-0002 は `Accepted provisionally` とし、
  LGPL-2.1 解釈について弁護士確認を Open Question に残す

## 実装方針

- bundle への binary 同梱は Tauri の resources 機能
  (`tauri.conf.json` の `bundle.resources`) で行う
- 同梱 binary の stage は CLI sidecar と同じ build script 拡張で
  (upstream download + SHA256 検証を build 時に実行)
- install 側は「bundle resources に binary があればそれを cache へ
  copy して利用」の fallback 順 (bundle → cache → download)

## 非対象 (本 change では実装しない)

- **DMG への binary 同梱そのものの実装** — ADR で方針を固定するが、
  実装 (build script 拡張・resources 追加・offline 経路) は法務確認
  後の別 change とする。本 change の成果物は ADR + openspec spec のみ
- Linux side (AppImage / deb) への同様の bundle — macOS DMG が決まれば
  同一方針の適用のみ。別途起票
- `nix-installer.sh` (upstream の shell wrapper) の再配布 — 本流の
  binary 配布と条件が異なるため、必要になった時に検討

## Impact

- **specs**: `managed-nix-bootstrap` に offline bundle に関する要件を追加
- **docs**: `docs/adr/0002-dmg-bundle-lgpl-redistribution.md` (新規)、
  `docs/adr/README.md` (一覧更新)、ADR-0001 OQ5 の解決参照
- **リスク**: 中 — ライセンス解釈を伴う。緩和策: (a) 無改変再配布に
  限定し改変の複雑性を排除 (b) source 提供は written offer で GitHub
  の永続性に依存しない archive も併記 (c) 弁護士確認を condition に

## Sources

- LGPL-2.1: https://www.gnu.org/licenses/old-licenses/lgpl-2.1.txt
- GPL FAQ (mere aggregation): https://www.gnu.org/licenses/gpl-faq.en.html#MereAggregation
- nix-installer LICENSE: https://raw.githubusercontent.com/NixOS/nix-installer/main/LICENSE
- upstream release: https://github.com/NixOS/nix-installer/releases
