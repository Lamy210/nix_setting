# Tasks: add-nix-tooling-evaluation-policy

> **スコープ:** 本changeはOSSの導入判断・PoC受入ポリシーを定義する**仕様**。新OSSの実行・CI変更・製品実装は承認後、独立したOpenSpec changeと実装PRで実施する。後続バックログは**本changeの完了条件には含まない**。

## 0. 仕様書PRの完成・承認（本PR）

- [x] 0.1 developのflake、devShell、Justfile、nix-unit tests、CI、OpenSpecと既存OSSを棚卸しする。
- [x] 0.2 未導入候補を優先PoC・条件付き評価・見送りへ分類し、既存実装との重複を排除する。
- [x] 0.3 セキュリティ・保守・コスト・OS互換性・CI被覆をHard Gateと定量評価に整理する。
- [x] 0.4 OSSごとに「誰が何をできるか / 期待メリット / 代替手段 / 反証」を設計に整理する。
- [x] 0.5 CLI/CI/インフラ/将来のGUI製品機能を分離し、不要な製品依存を禁止する。
- [x] 0.6 失敗/未実行/未サポートの区別、PoCエビデンス定義、negative tests、rollbackをspecへ追記する。
- [ ] 0.7 \`openspec validate add-nix-tooling-evaluation-policy --strict\` が最新HEADで成功する。
- [ ] 0.8 \`openspec validate --all --strict\` が最新HEADで成功する。
- [ ] 0.9 既存\`development-workflow\`、\`release-supply-chain\`、\`tool-resolution\`、\`platform-execution\`との矛盾をレビューする。
- [ ] 0.10 GitHub server-side required contextsとPR #324/#327/#299等との競合を再確認する。
- [ ] 0.11 有用性/目標値/保守・費用負担/リスクの承認を得る。未実測を成果とみなさない。
- [ ] 0.12 最新HEADで既存required CI成功、レビュー未解決0を確認し、Draft解除/マージを判断する。

## 1. このchangeのarchive（仕様承認・統合後のみ）

- [ ] 1.1 ポリシーの検証・レビューと統合後に別\`chore/archive-add-nix-tooling-evaluation-policy\` PRを作る。
- [ ] 1.2 新規canonical specへポリシー要件を同期し、候補OSSのPoCや導入を「実装済み」と誤記しない。

## 2. 後続バックログ（本changeの受入対象外／各々独立OpenSpec）

| 順位 | change候補 / ユーザー価値 | PoCの最低受入 | 対応するステータス |
| --- | --- | --- | --- |
| P0 | 既存nix-unit被覆確認：設定回帰を実機適用前に検出 | 既存test数・実CI呼出し・失敗fixtureを確認 | 既存維持/CI補強 |
| P0 | 計測baseline：今より速く/安くなったかを判断 | 同じSHA/lock/runnerでcold/warm、required critical path、Runner時間、CPU/RAM/bytesを記録 | 計測準備 |
| P1 | nix-diff：依存差分とキャッシュミスを説明 | 同一drvと意図した入力変更の両方を検証し、原因特定を確認 | 優先PoC |
| P1 | flake-checker：root/Node/devenvのpinを監査 | 各独立lockを列挙、未固定/古い/意図的pinの判定、false positiveを記録 | 優先PoC |
| P1 | nix-fast-build：多数のchecksを並列実行してCI待ち時間を削減 | 属性網羅表、negative test、既存required維持、critical path -25%かつRunner時間+20%以内を目標 | 優先PoC |
| P2 | sbomnix：Nix closureで依存リスクを説明 | syft/SPDXとの範囲・誤検知比較、既存release証明を維持 | 条件付き |
| P2 | ncps/niks3/Attic系：複数Runnerの再ダウンロード/再ビルドを減らす | ローカルstoreとの比較、署名、GC、障害・復旧、S3/DB総費用、trusted/untrusted境界 | 条件付き |
| P3 | nix2container：必要なLinux CLI OCIを効率的に生成 | 既存Dockerfileとruntime/サイズ/レイヤー/SBOM/転送比較、release DMGは不変 | 限定PoC |
| 保留 | Gradient/Hydra、Dynamic Derivations、Managed Nix provider変更、製品GUI統合 | 具体的需要・権限/互換・費用対効果が明確になった場合のみ別提案 | 見送り |

## 3. 後続PoCごとに必要な成果物（テンプレート）

- [ ] 独立changeにsource/license/release/immutable pin/互換system/現在の代替ツールを記録。
- [ ] 利用者シナリオ・現状課題・得られるメリット・採用しない条件を事前登録。
- [ ] 比較対象の同一SHA/lock/Runnerと、評価・実ビルド・キャッシュhit・runtimeを分けた検証表を登録。
- [ ] 成功/失敗/skip/unsupported/unknown、negative case、CPU/RAM/disk/bytes/time/API・費用を実測（不明はunknown）。
- [ ] Self-hosted Runner・PR write token・cache signing key・trusted-userの権限境界をレビュー。
- [ ] フェイルバック/GC/復旧の実験を実施し、rollbackを実行可能にする。
- [ ] Go/Conditional/No-Go/Deferの判断と承認者、実装PR、再評価トリガーをdecision logに保存。
- [ ] 採用済みと誤記せず各個別changeを実装・テスト・merge後にarchive。

> 未実施項目にチェックを付けない。既存required CIをskip/変更してgreenに見せない。
