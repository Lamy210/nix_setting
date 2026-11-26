# Nix tooling評価の記録例・異常系レビュー

以下はポリシーの運用確認用の机上ケースであり、PoC、ツール実行、正式採用の事実ではない。未計測項目はunknownを維持する。

| ケース | 根拠・入力条件 | 期待する記録 | 判定と理由 |
| --- | --- | --- | --- |
| 既存nix-unitの活用 | developのtests/nix-unit.nix、check.ymlのbootstrap-testに既存実行経路がある。サーバーrequired指定や被覆の完全性は別確認 | 既存活用、追加導入なし。追加のnegative fixtureやpinは別提案 | 既存活用の例、重複採用はNo-Go |
| nix-diff候補 | 既存差分診断との比較を計画。source pin、license、実行環境、測定、rollbackはunknown | PoC前の安全審査と比較計画を明示。実測未実施 | Defer。安全審査を通してからGo (PoC) を審査する |
| 安全審査不合格（仮想） | 外部PRの任意コードが信頼cacheの書込資格情報を要求 | Gate 0 fail、理由を記録、実行停止 | No-Go。性能点・実行の利便性で例外にしない |
| 未計測の性能主張（仮想） | 5成功runの探索結果、必須検証の一部が未実行 | 結果を探索扱い、未実行はskipped、実行方法も別記録 | Defer。最適化達成/本番採用と宣言しない |
| 独立lock欠落（仮想） | rootの検査は成功、追加テンプレートのlockは欠落 | rootのみverified、追加テンプレートのlock欠落を記録 | Defer。全Flake監査成功としない、意図的pinを自動更新しない |

## 机上の受入レビュー

- 重複OSS: 既存経路がある場合は新規導入済みとしてカウントせず、被覆改善を別にする。
- Gate 0不合格: untrusted write credentialのケースをスコアに関係なく拒否する。
- 未計測: 本番採用・CI最適化完了とは表示しない。
- 独立lock欠落: 未固定と検証成功を区別する。
- 正常なPoCの例: 取得元・license・pin・隔離・最小権限・予算・範囲・既存gate維持がレビューでpass、性能と互換性の実測のみunknownなら承認後に隔離PoCへ進める。本番採用は保留する。

これらはD7/D9およびdelta specとの机上整合確認であり、実環境でのnegative testやCI結果そのものではない。実装PRのレビュー・CIで最終受入を記録し、完了後に別archive PRを作成する。
