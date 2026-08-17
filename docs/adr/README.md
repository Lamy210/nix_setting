# Architecture Decision Records

SchneeForge のアーキテクチャ決定記録。Lightweight ADR (Michael Nygard 形式) で運用する。

## 一覧

| # | タイトル | Status |
|---|----------|--------|
| [0001](./0001-managed-nix-provider.md) | Managed Nix Provider に NixOS/nix-installer を採用 | Accepted provisionally |
| [0002](./0002-dmg-bundle-lgpl-redistribution.md) | DMG bundle 配布における LGPL-2.1 再配布条件の対応 | Accepted provisionally |
| [0003](./0003-configuration-source-model.md) | ConfigurationSource モデル (Release / Git / Local) | Proposed |

## 運用

- **ID**: `0001` から連番。ゼロ埋め 4 桁。
- **Status**: `Proposed` → `Accepted` / `Accepted provisionally` / `Rejected` / `Superseded by ADR-XXXX` / `Deprecated`。
- **新規作成タイミング**: アーキテクチャ・外部依存・セキュリティ・ライセンス・データモデル等、コードだけでは分からない決定を行うとき。
- **更新**: 決定を覆す場合は新 ADR で `Superseded` を明示。本文は履歴として残す。
- **場所**: `docs/adr/NNNN-kebab-case-title.md`。

## テンプレート

```markdown
# ADR NNNN: Title

Date: YYYY-MM-DD
Status: Accepted | Accepted provisionally | Rejected | Superseded by ADR-XXXX | Deprecated

## Context
(なぜこの決定が必要か。背景・制約・関係者)

## Decision
(決定内容。断言形で書く)

## Alternatives Considered
(検討した他の選択肢と却不理由)

## Consequences
- Positive: ...
- Negative: ...
- Neutral: ...

## Open Questions
(未解決事項。Final acceptance 条件があれば明示)
```
