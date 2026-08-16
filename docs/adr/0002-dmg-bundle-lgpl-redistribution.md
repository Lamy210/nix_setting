# ADR-0002: DMG bundle 配布における LGPL-2.1 再配布条件の対応

Date: 2026-08-16
Status: Accepted provisionally (弁護士確認を Open Question に残す)

## Context

Managed Nix の install は online download のみを想定している
(ADR-0001 Phase 1)。offline 環境 (閉域網・機密区域) での初回 install
を可能にするには、nix-installer binary を SchneeForge の配布物
(macOS DMG) へ同梱するのが自然な形だが、nix-installer は LGPL-2.1
であり binary の再配布には LGPL-2.1 の条件が発生する (ADR-0001
Open Question 5、issue #17)。

前提の整理:

- SchneeForge は nix-installer を subprocess で呼ぶ別プロセスであり、
  リンクしない。GPL/LGPL FAQ の "pipes, sockets and command-line
  arguments" に該当するため、SchneeForge 側 code へのライセンス伝染は
  無い (ADR-0001 の判断を維持)
- 問題は伝染ではなく**再配布**。LGPL-2.1 の section 3/4 が binary
  配布における義務を定める

LGPL-2.1 の binary 再配布に必要な条件 (section 3, 4 要約):

1. LGPL-2.1 本文と著作権表示の保持・利用者への表示
2. 対応 source の提供 (同梱または written offer — 3 年間有効)
3. 改変した場合の明示 (改変箇所の告知)
4. 利用者が library を差し替え可能であることの保障
   (object file と link 情報の提供。ただし**無改変かつ独立 binary の
   再配布には該当しない** — 差し替えは利用者が upstream から取得した
   binary で置き換えれば足りる)

上流 (NixOS/nix-installer) の実測 (2026-08-16):

- license: LGPL-2.1 (GitHub API `license.spdx_id`)
- release assets: `nix-installer-{aarch64-darwin,aarch64-linux,x86_64-linux}`
  + `nix-installer.sh` + `SHA256SUMS`
- source: release 毎に auto-generated `zipball_url` / `tarball_url` が
  常時存在 (tag 2.35.1 で確認)
- SchneeForge は manifest で version + arch 毎 SHA256 を pin 済み
  (upstream SHA256SUMS 由来。CI で SLSA provenance 検証)

## Decision

DMG bundle への nix-installer 同梱は以下の方針で行う:

1. **無改変再配布に限定する。** bundle する binary は upstream release
   asset の byte-identical copy とし、SHA256 が manifest pin 値と一致
   することを CI (release-artifact-check) で検証する。不一致は build
   fail。これにより「改変の明示」義務と差し替え保障の複雑性を排除する
2. **ライセンス同梱。** DMG 内 `Contents/Resources/licenses/nix-installer/`
   に LGPL-2.1 本文と著作権表示を置く。GUI の about / README に
   「nix-installer (LGPL-2.1) を含む」を明記する
3. **対応 source は written offer 形式。** release 毎に source 参照
   (upstream tag の tarball URL 集) を `SOURCES.md` として release
   asset へ添付し、要求があれば提供する。提供期間 (3 年) と手順を
   RELEASE.md に明記する。GitHub の archive 永続性に依存しないよう、
   archive.org 等の外部 archive の URL も併記する
4. **offline install 経路。** bundle 内 binary を SHA256 検証の上
   cache へ取り込んで使う。解決順は bundle → cache → download とし、
   bundle / cache に存在する場合は network access を行わない

ADR-0001 とは独立した決定として扱う (provider 選択は不変)。

## Alternatives Considered

### A. bundle を行わず online download のみ維持 (現状)

- ◎ ライセンス義務が一切発生しない。実装も現状のまま
- ✗ offline 環境の初回 install が不可能なまま。issue #17 の目的が
  達成できない。cache は初回 download を済ませた環境でしか効かない

### B. bundle せず「offline kit」を別配布 (zip で binary + 手順書を配る)

- ◎ SchneeForge の配布物に含まれないため DMG 側の義務は増えない
- ✗ zip 配布自体が LGPL-2.1 の binary 再配布に該当するため義務は同等
  に発生する。加えて install 手順が手動化され UX が劣化する。却下

### C. source から self-build して同梱 (改変無しでも自前 build)

- ◎ build 時の検証を自前で挟める
- ✗ upstream release と byte 異なる binary になると「無改変」の主張が
  弱まり、対応 source の提供義務を自ら満たす体制 (build script の
  公開・再現環境の提供) が必要になる。検証も複雑化。却下

### D. nix-installer 以外の MIT license installer へ乗り換え

- ◎ ライセンス義務が消える
- ✗ ADR-0001 の Alternatives 再検討と同じ理由 (receipt / plan /
  ownership ledger の前提を満たす installer が他に無い) で却下

## Consequences

### Positive

- offline 環境の初回 install が可能になる (issue #17 の解決)
- 無改変 + SHA256 gate により、ライセンス義務を「本文同梱 + クレジット
  表示 + written offer」の最小セットに留められる
- bundle binary の検証が supply-chain 保護 (manifest pin) と同じ仕組み
  で済む

### Negative

- DMG size が増える (nix-installer aarch64-darwin 約 20MB 程度)
- release 毎に `SOURCES.md` 作成と written offer の管理 (3 年間) が
  必要になる
- 弁護士確認を経るまで Status が provisional に留まる

### Neutral

- SchneeForge 側 code の license は MIT のまま (伝染しない)
- Linux side (AppImage / deb) への同梱は macOS と同一方針の適用と
  なる (別途起票)

## Open Questions

1. **弁護士確認 (Final acceptance 条件)**: (a) 無改変 binary の bundle
   における section 4 (差し替え保障) の解釈 — 独立 binary の再配布では
   適用除外と読めるか、(b) written offer の文面 (英語・日本語)、
   (c) SOURCES.md の外部 archive URL 併記で十分か。確認結果次第で
   本 ADR を `Accepted` へ昇格させる
2. **DMG size 増の受容判断**: ユーザー視点の許容範囲 (install 済み
   環境にとっては不要な file)。online 専用 build の併用が必要か
3. **upstream license 変更時の追従**: nix-installer が LGPL-2.1 以外へ
   変更した場合の検知・対応 (upstream-nix-installer.yml への
   license check 追加を検討)
