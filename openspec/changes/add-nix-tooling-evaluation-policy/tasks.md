# Tasks: add-nix-tooling-evaluation-policy

## 0. 仕様調査・承認（本PRの対象）

- [x] 0.1 `develop` の `flake.nix`、devShell、`justfile`、`tests/nix-unit.nix`、主要CI、OpenSpec規約を読み、既存OSSを棚卸しする。
- [x] 0.2 優先候補（nix-fast-build / nix-diff / flake-checker / sbomnix / niks3 等）の用途と既存OSSとの差分を文書化する。
- [x] 0.3 採用段階、代替案、実行境界、セキュリティ要件、性能指標、rollbackをdesignとdelta specへ記述する。
- [ ] 0.4 `openspec validate add-nix-tooling-evaluation-policy --strict` と `openspec validate --all --strict` の**最新HEADでの成功**を確認する。
- [ ] 0.5 `development-workflow`、`release-supply-chain`、`tool-resolution` のspecと重複・矛盾がないかPR reviewする。
- [ ] 0.6 提案をレビュー・承認する。承認まで実装、導入、CI設定変更を開始しない。

## 1. P0: 計測準備・既存ツール活用（承認後の別PR）

- [ ] 1.1 `justfile test` / `tests/nix-unit.nix` がrequired CIに含まれているか調査する。未実行の場合は既存ツールを使用した追加案をレビューする。
- [ ] 1.2 既存 `ci-required` およびserver-side required contextsと、実行/静的評価の対象platformを照合する。
- [ ] 1.3 同一commit/flake.lock/Runner classでcold/warm基準値を取得する（目安: 条件別5回以上の成功run）。
- [ ] 1.4 `docs/testing/ci-workflow-timing.md` の集計を用い、critical path、runner時間合計、CPU、peak RSS、ストレージ/転送量を保存する。
- [ ] 1.5 ベースラインにmissing/unknownがある場合は追加計測し、推測値で成功判定しない。

## 2. P1: 低リスクOSSのPoC（各々独立PR）

- [ ] 2.1 `nix-diff`: 固定リビジョンとライセンス・OSを検証し、2つのderivationの差分をread-onlyで再現する。
- [ ] 2.2 `flake-checker`: 古いNixpkgs入力/固定理由の監査をnon-requiredに追加し、誤検知の判定基準を整える。
- [ ] 2.3 `nix-fast-build`: checks属性の網羅とビルド・評価の違いを確認し、非blocking PoCで測定する。
- [ ] 2.4 P1で追加するすべてのツールはcommit/hash pin、権限、ログのシークレット混入、CPU/RAM/時間を審査する。
- [ ] 2.5 同一HEADのrequired `flake-check`、`macos-check`、`rust-check`、`release-artifact-check`、OpenSpec gateを維持して比較する。
- [ ] 2.6 目標未達・誤検知・環境非互換ならPoCを停止し、そのツールの設定/lock変更のみをrollbackする。

## 3. P2: SBOM・キャッシュ比較（別OpenSpec/実装PR）

- [ ] 3.1 `sbomnix` の対象Nix closure、生成SBOM形式、既存syft/SPDX/cosignとの重複・差異を計測する。
- [ ] 3.2 sbomnixの脆弱性推定をclosure/依存形式でtriageし、false positiveを説明可能にするまではnon-requiredを維持する。
- [ ] 3.3 local store vs `ncps` vs `niks3` vs Attic系を、hit率/転送量/重複排除/保守状況で比較する。
- [ ] 3.4 S3互換cacheの署名鍵、OIDC、read/write分離、GC、DR、容量制限、料金見積を別のインフラ設計としてレビューする。
- [ ] 3.5 課金または運用負担が便益を上回る場合は既存Runner local storeのまま継続する。

## 4. P3: OCI・研究候補（任意・個別承認）

- [ ] 4.1 `nix2container` をCLIの隔離テストに限定し、既存Dockerfileやリリース/DMGを変更せずレイヤー・サイズ・脆弱性検査を比較する。
- [ ] 4.2 `Gradient`/`Hydra`、Dynamic Derivations、Nix実装変更、system-manager、LLM agent package導入は独立の採用審査まで見送る。

## 5. 検証・リリース・アーカイブ

- [ ] 5.1 必須CIとplatform matrixに対しevaluation / build / runtime / installを別々に検証する。
- [ ] 5.2 最新HEADのrequired checks success、review未解決0、rollback手順を確認して各実装PRを個別にsquash mergeする。
- [ ] 5.3 障害/誤検知/費用/計測値をdecision logに記録し、採用・保留・不採用を更新する。
- [ ] 5.4 実装PR完了後にのみ、別 `chore/archive-add-nix-tooling-evaluation-policy` PRでOpenSpecをarchive/syncする。

> この仕様PRには実装変更を含めない。未実行の検証はチェック済みにしない。
