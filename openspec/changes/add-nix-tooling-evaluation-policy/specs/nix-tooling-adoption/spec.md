# nix-tooling-adoption Specification Delta

## ADDED Requirements

### Requirement: Nix OSS導入の重複防止と選定記録

SchneeForge の開発基盤にNix関連OSSを追加する場合、提案者は既存 `flake.nix`、`devShells`、`justfile`、tests、OpenSpec、CIにある同等機能を調査し、候補ごとの `既存維持` / `PoC` / `条件付き採用` / `見送り` を記録しなければならない (MUST)。ライセンス、保守状況、対象OS、追加コストが確認できないものは `unknown` とし、無根拠に本番採用と判定してはならない (MUST NOT)。

#### Scenario: 既存nix-unitを再評価する
- **WHEN** 開発用Nixテストツールの導入計画を立てる
- **THEN** 既存の `tests/nix-unit.nix` と `justfile` の実行経路を調査する
- **AND** `nix-unit` を未導入OSSとして二重に追加しない

#### Scenario: 新しいOSSの保守状況が不明
- **WHEN** OSSの最新commit、互換性、ライセンス、脆弱性情報を確認できない
- **THEN** その項目を `unknown` と記録する
- **AND** 本番の既定機能として自動有効化しない

### Requirement: 優先OSS候補は承認付きPoCから導入する

新規採用候補 `nix-fast-build`、`nix-diff`、`flake-checker` は、提案承認後に独立した実装PRと観測可能なPoCで検証しなければならない (MUST)。PoCの成功は、SchneeForge製品への自動配布またはrequired CIへの昇格を意味してはならない (MUST NOT)。

#### Scenario: nix-fast-buildを試験導入する
- **WHEN** `nix-fast-build` によるビルド高速化を検討する
- **THEN** pinされた入力でnon-requiredまたは手動ジョブを追加する
- **AND** 現行required `nix flake check` を残して比較する
- **AND** 性能と実行属性の比較結果なしにrequiredを置換しない

#### Scenario: flake-checkerを初めて実行する
- **WHEN** lockfileやNixpkgs追従状況を監査する
- **THEN** 既存inputの固定理由とツールの判定根拠を記録する
- **AND** 警告だけを根拠にlockを自動更新したりPRを失敗扱いしたりしない

### Requirement: 既存required検証の被覆を低下させない

Nix関連ツールの導入・高速化・キャッシュ適用は、既存 `development-workflow` の required contexts、Linux Home Manager評価とrealization、macOS stable matrix、release artifact実物検証、Rust品質検査、OpenSpec strict validationを維持しなければならない (MUST)。新しいジョブが一部の属性だけを成功させても、既存gateを成功と読み替えてはならない (MUST NOT)。

#### Scenario: Flake checks以外の受入が残る
- **WHEN** `nix-fast-build` が `checks` 属性のビルドに成功する
- **THEN** Linux Home Manager `activationPackage`、nix-darwin system、release CLI/DMGなどの別経路は従来どおり検証する
- **AND** `ci-required` candidateと既存required contextの集約規約を変更しない

#### Scenario: PoCジョブが失敗またはskippedになる
- **WHEN** 新しいnon-required PoCジョブがfailure、cancelled、skippedとなる
- **THEN** PoC結果は失敗・未完了として保存する
- **AND** その結果を既存required checkの成功や検証完了の証拠にしない

### Requirement: OSS導入は同一条件の再現可能な比較で判断する

性能を改善するOSSは、導入前後について同一commit SHA、`flake.lock`、runner、system、toolchain、キャッシュ状態でbaselineと候補を比較しなければならない (MUST)。queue時間とrunner実行時間、critical pathとrunner時間合計、cold/warm cacheの結果を区別して報告しなければならない (MUST)。既存のCI最適化要件で定めた **critical-path 25%以上短縮、runner-time増加20%以内**の目標を満たさない場合、最適化完了と宣言してはならない (MUST NOT)。

#### Scenario: 同一条件でnix-fast-buildを評価する
- **WHEN** 並列ビルドのPoC結果を報告する
- **THEN** baselineとPoC双方のrun ID、platform、評価/ビルド対象属性、CPU秒、peak RAM、経過時間、転送量、成功/失敗を示す
- **AND** cold/warm cacheを別々に比較する
- **AND** 未計測項目を `unknown` と表示する

#### Scenario: 速度は改善してもrunner時間が過大になる
- **WHEN** required critical pathが25%以上短縮されてもrunner合計時間が20%を超えて増加する
- **THEN** 当該案を最適化完了として扱わない
- **AND** 同時実行数、重複実行、計測条件を再設計する

### Requirement: OS・CPU別の検証段階を区別する

OSS採用の受入表は `aarch64-darwin`、`x86_64-linux`、`aarch64-linux` の対象ごとに `evaluation`、`realization`、`runtime smoke`、`install acceptance` を区別しなければならない (MUST)。WindowsはWSL2 Linux helper経由という既存境界を維持し、Windows native Nix実行やx86_64-darwin正式対応を暗黙に宣言してはならない (MUST NOT)。

#### Scenario: Linux上でmacOS出力を評価する
- **WHEN** Linux上でDarwin向けderivationの評価のみが成功する
- **THEN** 状態を `evaluation verified` と記録する
- **AND** macOS実機のビルド・起動・インストールの成功とみなさない

#### Scenario: WSL2でNixツールを使う
- **WHEN** Windows hostからNix関連処理を実行する
- **THEN** 既存Windows launcherからWSL2 Linux helperへ委譲する
- **AND** ネイティブWindows側でNixの実行環境を新設しない

### Requirement: 外部OSSとキャッシュのサプライチェーンを保護する

新規OSSをCIまたは運用で実行する場合、承認された固定version/commitと再現可能な取得経路、ライセンス審査、権限境界を必要とする (MUST)。共有バイナリキャッシュは署名鍵・公開鍵・substituter・read/write credentialの境界を審査しなければならず (MUST)、untrusted PRに署名鍵やwrite tokenを渡してはならない (MUST NOT)。

#### Scenario: 外部fork PRでキャッシュを使う
- **WHEN** 信頼されないPRの検証ジョブが実行される
- **THEN** 読み取り側は許可済みの署名検証と信頼設定だけを使う
- **AND** write token、cache signing key、リリース用OIDC権限を提供しない

#### Scenario: 署名不一致・不正なストア取得が発生する
- **WHEN** substituteの署名検証に失敗する
- **THEN** 信頼できない成果物を採用しない
- **AND** 許可済み上流・従来ビルドへ戻し、検証失敗を記録する

### Requirement: SBOM PoCは既存リリース保証を置換しない

`sbomnix` 等を開発時のNix依存監査に使う場合、既存 `release-supply-chain` のcosign/SLSA/SPDX生成・自己検証・配布要件は維持しなければならない (MUST)。脆弱性推定はpurl/CPE・実依存と照合し、誤検知を審査するまでは新規requiredゲートとしない (MUST NOT)。

#### Scenario: sbomnixによるNix依存SBOMを生成する
- **WHEN** Nix closureの依存関係を検査する
- **THEN** 対象store path、入力ref、SBOM形式、既存release SBOMとの差異を記録する
- **AND** 既存の署名済みrelease artifactを変更しない

### Requirement: キャッシュ・ビルド基盤導入には費用と復旧計画を必要とする

`niks3`、`ncps`、Attic系、分散CIなど永続サービスを導入する場合、導入前に保存容量、API回数、ネットワーク転送量、Runner時間、バックアップ/DR、GC、監視、秘密鍵ローテーションを含む計画を示さなければならない (MUST)。不明な料金・性能は `unknown` とし、費用と運用負荷が正当化されない場合は現行Runnerローカルキャッシュを維持しなければならない (MUST)。

#### Scenario: S3キャッシュを検証する
- **WHEN** `niks3` の共有キャッシュを提案する
- **THEN** ローカルのみの場合とS3導入時のhit率、転送量、ストレージ、PUT/GET、障害時挙動を比較する
- **AND** 本番の信頼鍵とstore GCをPoCから変更しない

#### Scenario: キャッシュ破損やサービス停止が発生する
- **WHEN** キャッシュデータを取得できない、またはGCで必要な成果物が失われる
- **THEN** 信頼済み上流またはソース再ビルドで復旧できることを検証する
- **AND** requiredの検証をskipしない

### Requirement: PoC採用・撤退とOpenSpecライフサイクルを追跡する

各PoCにはdecision log、受入結果、障害・誤検知、rollback手順、未確認事項、承認者・後続PRを記録しなければならない (MUST)。提案承認前に実装を開始せず、実装後に別のarchive PRでcanonical specを同期しなければならない (MUST)。

#### Scenario: PoCの採用条件に届かない
- **WHEN** 性能目標、セキュリティ審査、必須検証、コスト条件のいずれかを満たさない
- **THEN** PoCを不採用または再検証と記録する
- **AND** 非required workflow・設定・lockを独立に戻す手順を用意する

#### Scenario: 全条件を満たして本番採用へ進む
- **WHEN** 実測・機能差分・安全性がレビューされ提案が承認される
- **THEN** ツール単位の実装PRで本番採用する
- **AND** 最新HEADのrequired checksとrelease gateを満たした後にmergeする
- **AND** 実装merge後に別のOpenSpec archive/spec-sync PRを作成する


### Requirement: 候補採用前にHard Gateと根拠付き評価を実施する

候補を本番用ツールチェーンまたはrequired CIに昇格させる前に、採用責任者はライセンス/配布条件、固定取得元、依存の保守状況、利用可能なセキュリティ情報、対象OS、導入費、rollbackを確認しなければならない (MUST)。重要項目の判定が `unknown` の場合は本番採用を保留しなければならない (MUST)。評価点が高くてもセキュリティ・既存CI非回帰を免除してはならない (MUST NOT)。

#### Scenario: セキュリティHard Gateを満たさない
- **WHEN** 外部OSSがuntrusted PRの書込み資格情報を必要とする、または実行ソースを固定できない
- **THEN** 本番採用の結論は `No-Go` または `Defer` となる
- **AND** スコアが高くても例外的に自動導入しない

#### Scenario: OSSが高評価でも互換性が未知
- **WHEN** macOS ARM64・Linux x86_64など採用対象で実行確認が不足する
- **THEN** 当該platformの検証状態を `unknown` と記録する
- **AND** 全OS対応済みとして公開しない

#### Scenario: 正式な採用可否を判断する
- **WHEN** PoCが完了し採用会議・PR reviewでGo/No-Goを決める
- **THEN** Gate 0結果、評価軸の採点と根拠、代替案、決定者、再評価条件を記録する
- **AND** 成功したツールだけを選別して実行・失敗結果を隠さない

### Requirement: Flake入力監査は独立lockと例外pinを識別する

Flake入力の健全性検査はルートだけでなく、独立して依存を解決するNode・devenv等のテンプレートも棚卸ししなければならない (MUST)。lockfileが未作成・欠落している場合は明示的に `unlocked` または `unknown` と記録しなければならない (MUST)。更新の遅延だけで即時の自動更新やブロッキングを行ってはならない (MUST NOT)。

#### Scenario: rootのみのflake-checker実行が成功する
- **WHEN** ルートの `flake.lock` を検査するOSSが成功を報告する
- **THEN** 結果をルートの検査成功として記録する
- **AND** `templates/node/` と `templates/devenv/` の依存監査が完了したとは表示しない

#### Scenario: セキュリティ修正のため古い入力が固定されている
- **WHEN** Nixpkgs inputが経過日数閾値を超える一方、別途固定したruntime/依存closureがある
- **THEN** 例外の理由、実体runtime検査、更新可能な候補を個別に審査する
- **AND** 経過日数のみを根拠に修正済みランタイムを自動的に置換しない

### Requirement: ビルド高速化の結果は属性と実行ステージで検証する

CI高速化PoCは対象attributeのexpected/evaluated/built/skipped/unsupported/failedを記録しなければならない (MUST)。`nix-fast-build` がchecks属性に成功した場合でも、`nix flake check` のoutputs評価、Home Manager・nix-darwin、実配布CLI/DMG、およびインストール受入の完了を意味してはならない (MUST NOT)。

#### Scenario: キャッシュヒットによってビルドが省略される
- **WHEN** `--skip-cached` を利用したPoCがキャッシュからビルド結果を再利用する
- **THEN** 当該属性の状態を `cached/substituted` として証拠に残す
- **AND** そのrunでソースビルドを再実行したという記録にしない

#### Scenario: unsupported属性をskipする
- **WHEN** `nix-fast-build` が対象systemで未サポートの属性をskipする
- **THEN** `unsupported` として結果を保持する
- **AND** 全属性のruntime/realizationが成功したものとしてrequired gateを置換しない

#### Scenario: read-only PoCでremote buildを提案する
- **WHEN** `--remote`、`--store` または成果物uploadを追加したい
- **THEN** SSHの権限、Nix trusted-user、キャッシュ書込み/鍵の脅威モデルを別レビューへ送る
- **AND** 初期の非特権PoCで勝手に有効化しない

### Requirement: CIとSelf-hosted Runnerの実行信頼境界を守る

第三者OSS・任意PRからのコード実行を伴うCIは、信頼されたコードとuntrustedコードの権限を分離しなければならない (MUST)。`pull_request_target`、`workflow_run`、コメント起点の高権限workflowでuntrusted checkoutやartifactを実行してはならない (MUST NOT)。再利用型self-hosted Runnerに外部PRの任意コードを実行させる場合は、同等の隔離と復旧が実測で証明されるまで許可してはならない (MUST NOT)。

#### Scenario: forkのPRからPoCを実行する
- **WHEN** forkのPRで外部OSSの検証コードが実行される
- **THEN** 書込み権限・シークレット・内部ネットワークへのアクセスを持たない隔離Runnerを使用する
- **AND** 既存の永続self-hosted Runnerには信頼できないPRコードを直接投入しない

#### Scenario: workflow_runがPR artifactを受け取る
- **WHEN** 権限を持つ後続workflowがuntrusted PRからのartifactを取得する
- **THEN** artifactをデータとして検証し、実行可能なコードとして実行しない
- **AND** 署名/昇格確認が終わる前にキャッシュやreleaseへpublishしない

### Requirement: キャッシュの失効・障害・費用超過を運用仕様で制御する

共有バイナリキャッシュを検証する場合、候補の読み取り・書き込み経路、`trusted-public-keys`、substituter、署名失敗時の挙動、credential rotation、GC、容量上限、バックアップ/DR、転送量・API課金を定義しなければならない (MUST)。キャッシュ障害を理由に署名検証を無効化してはならない (MUST NOT)。

#### Scenario: cache signing keyが漏洩する
- **WHEN** cache signing keyの漏洩が判明または疑われる
- **THEN** 書込み停止・信頼鍵の失効/交換・影響したstore pathの隔離/再取得を実施する復旧手順を発動する
- **AND** 署名検証を無効化せずに既存の信頼済みビルド経路へ戻す

#### Scenario: キャッシュの容量・転送費用が想定を超える
- **WHEN** 監視値が承認した容量・API・転送量・費用の上限を超える
- **THEN** PoCのuploadを停止し、現行のローカルstore利用へ戻す
- **AND** 未確認の無料枠や超過単価を0円と仮定しない

#### Scenario: GCで必要な成果物が消失する
- **WHEN** GC後に検証対象のstore pathが取得不能となる
- **THEN** 信頼済み上流またはピン留めしたソースから再ビルドできることを確認する
- **AND** requiredのチェックを省略せず障害を記録する

### Requirement: PoC判定は追跡可能な証拠とNegative testを伴う

各ツールのPoCはツール固定version、repo/lock SHA、Runner/OS、対象属性、実行ステージ、cold/warm、成功/失敗/skip、CPU/RAM/ストレージ/転送量と測定方法、費用または `unknown`、decision logへのリンクを保存しなければならない (MUST)。実際に失敗を検出できるnegative testを少なくとも1つ含まなければならない (MUST)。

#### Scenario: 評価が成功しテストビルドが失敗する
- **WHEN** Nixのevaluationが成功したがderivationのbuildが失敗する
- **THEN** `evaluation=verified`、`realization=failed` を別々に記録する
- **AND** 一括の `success` と表示しない

#### Scenario: ベンチマークに必要な実行結果が不足する
- **WHEN** 比較条件ごとの成功runが5件未満、または別のRunner/lockで取得される
- **THEN** 結果を探索・参考値として扱う
- **AND** 根拠なしに性能目標達成や本番採用を宣言しない

#### Scenario: 失敗・skipが発生したPoCを報告する
- **WHEN** PoCでfail、skip、unknownが含まれる
- **THEN** 成功runだけでなく全結果数と原因を残す
- **AND** 失敗した属性の除外だけで検証範囲を狭めない

### Requirement: 導入評価ポリシーと個別OSS導入変更のライフサイクルを分離する

`add-nix-tooling-evaluation-policy` は選定プロセスと受入条件を定義する変更であり、各OSSの導入・有効化・CI最適化はそのツールの独立したOpenSpec changeで追跡しなければならない (MUST)。ポリシーのarchiveをすべての将来PoCの完了まで無制限に延期する必要はなく、ポリシー確定/実装後に既存のarchive規約に従わなければならない (MUST)。

#### Scenario: 選定ポリシーが承認される
- **WHEN** 本PRのstrict validation、レビュー、required gateがすべて完了する
- **THEN** ポリシーの仕様・今後の実装gateを確定する
- **AND** 任意OSSの実装が完了したとは扱わず、個別PoC changeを新規に開始できる

#### Scenario: 将来のPoCが未完了のままポリシーが確定する
- **WHEN** ポリシー自体の実装・文書化が完了し後続PoCが残る
- **THEN** このchangeを既存のOpenSpec archive規約に沿って別PRで同期できる
- **AND** 未完了のOSS導入タスクを完了済みとして閉じない
