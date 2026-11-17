## Context

Issue #335 の同一run 43598799237ではroot uv drv `02ifk7b9cjns5r8b3i2rb8cx09ys8bp7-uv-0.13.5.drv`、template uv drv `gzh922ivw0cm39wb31c8vrilfkxm34lp-uv-0.13.5.drv` のbuild開始を確認。root nixpkgs pinは `fe13242281a67641c0102cdb1087f0ebe586c7df`、Python template pinは `2fcb964de67fcf60b43471c55d5d99e61a9ccb5a`。これは調査時点の事実であり、PoC時は正確なcommitとlockで再確認する。

3成功runの中央値はroot devshell-smoke 1577秒、template-check (python) 1191秒。異なるworkload SHA・cache状態未確認・各n=3の探索値であり、uv単体時間やCI全体の短縮率ではない。

## Goals / Non-Goals

- 診断: 実際に各shellが選択するuvとそのderivation/入力グラフを識別し、同一versionでも差があることを追跡する。
- 比較: rootと独立templateの現行pinと、同一nixpkgs revisionへの整合候補を分けて測る。security override維持・配布単体で動作を条件とする。
- 初期PoC: x86_64-linuxの隔離環境のみ。新規永続サービス、shared cache書込み、remote builder、外部PR実行、高権限workflow、製品のapply/rollback変更は非対象。

## Decisions

### D1: workloadと診断harnessを分離

baseline workload SHA、全独立Flakeのlock SHA-256、harness SHA、候補差分を記録。既存のセキュリティpinを維持し、#299/#324/#334との重複・退行をreviewする。候補は使い捨ての書込み可能checkoutに限定し、元のcheckoutでlock更新やGCを行わない。実装前にGate 0の固定取得元・ライセンス・隔離・費用上限・停止条件を記録。承認されていない有料Runnerやサービスを追加しない。

### D2: 実体を特定してから比較

rootは `devShells.x86_64-linux.python`、templateは配布時と同じファイル一式を独立ディレクトリにコピーした `devShells.x86_64-linux.default`。親repoを前提にしない。Nixの固定済み環境でshellの入力derivationを調べ、uv候補とvendor入力を列挙し、`nix derivation show` 等の標準Nixコマンドで差分を取得する。利用可能なCLI構文は実装時に対象Nix versionで検証する。実行されたuvの絶対パス・version・選択されたclosureの関係を別途確認し、候補名の部分一致やversion文字列だけで特定しない。0件/複数候補/欠損はunknownまたはfailedとし、false PASSを禁止。診断のread-onlyとはworkloadソースや設定・lockを変更しないことを指し、Nixによる評価結果のstoreへの書込みまで否定するものではない。realizationは承認された隔離環境でのみ別ステージとして行う。

差分は直近の入力（nixpkgs revision、src、cargoDeps、stdenv、Rust toolchain、その他入力drv）から比較する。直接入力の差を原因候補とし、単一因子の因果関係は他条件を固定して検証する。drvが同一でも別Runner間でstoreが共有されなければ再利用を保証しない。JSONの環境値・URL・絶対home pathはallowlistとマスク処理で保護し、秘密情報を報告へ出さない。

### D3: 比較を段階化

| 案 | 計測 | 採否 |
| --- | --- | --- |
| A 現状pin | baseline・全入力差分 | 必須 |
| B 独立template pin整合 | x86_64-linuxで独立lock維持、uv/Python/ruff/pyright実行、closure差分 | 単一因子実験、候補のみ |
| C 共通recipe・cache・Runner集約 | 後続changeで同一drv/運用上の再利用可能性を実証 | 本changeの実装対象外 |

Bで根本の依存関係やpackage recipeが変わる可能性を記録し、lockのpin変更以外の変更を混ぜない。現行のuv 0.13.5 backportの安全性・実体versionと必要runtimeを維持。最新版を理由に自動置換しない。

### D4: 計測と判定

#330のD9 evidence recordに準拠し、対象属性、run/attempt/job URL、Runner/system、lock/workload/harness SHA、cold/warm、execution method、stage/outcome、duration/runner秒/CPU/RSS/disk/転送/費用またはunknown、rollback/decisionを記録。各条件のcold/warmそれぞれの成功run10件以上を正式比較の目安とし、まず少数の予備計測で測定手順を確認。別Runner/負荷/lock scopeが比較不能なら探索扱い。失敗・キャンセル・skip・unknownを保存する。費用未承認または上限到達は停止してDeferとする。

個別の目標は、比較可能なペアのdevshell-smokeとPython templateのrunner elapsed合計の中央値をbaseline比20%以上削減すること。各runの対応する2 jobを足してから中央値を算出し、p95と失敗率も提示。独立template・security・必須CIの非回帰は必須。CI全体の完了判定は別の既存目標（critical path -25%以上・runner time +20%以内）に従う。CIの実費、キャッシュhit率、uv単体build時間は測定した場合のみ報告。作成→開始の時間はrunner allocation以外を含む。

### D5: 検証と安全性

既存CI gate、macOS15/26、Linux x64/ARM64、Managed Nix、release-artifact/CodeQLは維持。診断fixtureで同一/異なるdrv、候補0件/複数、lock欠落、malformed JSON、version不一致、マスキングを確認。異常を検出するテストはfake successを許さない。対象外systemでのevaluationとrealization/runtimeを混同しない。新規OSS利用は別のGate 0審査。

## Risks / Trade-offs

- pin整合してもpkgs configや依存が異なるためdrvが一致しない → D2の比較で確認。候補不採用も成果とする。
- driverの名称検索が別のuvを誤選択 → 実体パス/入力との対応、曖昧時fail-closed。
- cold/warm混在やrunner待機をbuild短縮と誤認 → 隔離した同条件と独立指標。
- lock更新でbackport喪失や配布単体で動作不能 → 候補を撤回し、既存lockを維持。
- 計測コスト増大 → 事前承認された上限と停止条件、少数の予備計測から開始。
- evidenceに秘密情報を含める → allowlist、マスキング、機密入力の負例テスト。

## Migration Plan

1. 提案のstrict validation・承認、#332運用との整合を確認する。
2. 独立PRで診断・fixture・手順を実装。通常CIを通し、read-only契約を検証。
3. 許可済み隔離環境でbaseline/候補を計測。結果をIssue #335に記録。恒久変更なし。
4. 次の変更は別OpenSpecで採否を決定。診断実装のmerge後、別archive PRでcanonical specを同期。将来の最適化の完了までarchiveを延期する必要はない。
5. rollback: 診断PRのrevert。候補checkoutと作成済み一時出力のみ破棄。元のlock、trusted keys、shared store、既存CIは変更しない。

## Open Questions

- Nixの実バイナリ/version、実測runner・費用上限・独立templateでのexact package identityは未確認。承認・計測に先立ち確認する。
- 実際にpin整合で同一drvになるか、CI費用・クリティカルパス改善が成立するかはunknown。
