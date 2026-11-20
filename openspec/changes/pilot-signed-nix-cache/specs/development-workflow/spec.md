## ADDED Requirements

### Requirement: Signed cache pilot authorization boundaries

共有 Nix cache の実験はローカル署名検証、fresh runner取得、remote cache、CI採用を独立して承認しなければならない (MUST)。

#### Scenario: ローカル実験を承認する
- **WHEN** Phase Aが承認される
- **THEN** native macOS ARM64の使い捨てRunnerに限定し、固定revision・drv・outを検証する
- **AND** 外部cacheへの送信や本番資格情報の利用を行わない

#### Scenario: 固定成果物が一致しない
- **WHEN** 評価結果、Git状態、実行環境のいずれかが承認済み条件に一致しない
- **THEN** 署名を開始せず中断し、固定値を自動変更しない

### Requirement: Signed cache pilot acceptance evidence

署名付きcacheは実機での内容・署名・独立したnegative controlを検証しなければならない (MUST)。模擬CLIのみを実機成功と扱ってはならない (MUST NOT)。

#### Scenario: ローカルcacheを検証する
- **WHEN** Phase Aで対象をrealizeし一時cacheへcopyする
- **THEN** rootと依存closureのNAR hash、size、referencesを照合する
- **AND** 正鍵で検証成功し、別鍵でrootとclosureの双方が信頼性エラーになることを確認する

#### Scenario: 新版のPhase A証跡をレビューする
- **WHEN** 新版PoCの両macOSレーンの実機観測が提出される
- **THEN** 独立に承認した実装commitと全実行・検証ファイルのdigestを照合する
- **AND** 固定対象checkout、原ログ、原観測JSON、署名検証結果と実機operator recordの対応を検証する
- **AND** 両レーンの独立実行とclosureの同一性を確認し、人間の受入記録が揃うまでPhase Bを許可しない

#### Scenario: 旧版又は異なる実装の証跡が提出される
- **WHEN** 旧v6 recordが新版の実機証跡として提出される、承認済み実装identityと不一致になる、又は実行identityと原データの対応が不明である
- **THEN** 新版の受入を拒否し、承認済みの実装と実際の原証跡で再検証する
- **AND** ローカル観測の整合性検査だけを承認や実機実行の証明に使わない

#### Scenario: 別Runnerの取得を検証する
- **WHEN** Phase Bを承認してfresh consumerで実行する
- **THEN** 対象が未realizeであり承認されたcacheから取得した証跡と取得後closureの一致を記録する
- **AND** 誤鍵および破損NARを隔離したfixtureで拒否する

#### Scenario: 証跡が不完全である
- **WHEN** 実取得経路、署名拒否、内容検証又はidentityの証跡が不足する
- **THEN** 実験結果を未確認又は失敗として記録し次のPhaseを許可しない

### Requirement: Remote cache publisher isolation

remote cacheの署名秘密鍵と書込権限は信頼済みpublisherに限定しなければならない (MUST)。未審査closureの送信を禁止する (MUST NOT)。

#### Scenario: PRがcacheを利用する
- **WHEN** PR consumerがcacheにアクセスする
- **THEN** 読取のみ、署名鍵なし、信頼済み公開鍵での検証を維持する

#### Scenario: R2の実験を開始する
- **WHEN** Phase Cを承認する
- **THEN** 配布条件、private bucket、資格情報、予算、GCとrollbackをレビューし記録する
- **AND** 未決の項目がある場合はuploadを開始しない

### Requirement: Cache adoption preserves CI coverage

キャッシュ導入は両macOSレーン・既存CI・release検証の必須条件を維持しなければならない (MUST)。

#### Scenario: cacheが利用できない
- **WHEN** cache miss又は通信障害が発生する
- **THEN** 通常ビルドと検査を維持する
- **AND** 署名不正を単なるmissとして成功扱いしない

#### Scenario: CI導入を評価する
- **WHEN** matched sampleを各条件・各レーン10成功以上収集する
- **THEN** gate elapsed、runner time、転送、費用と失敗を比較する
- **AND** 承認済みcache固有閾値と適用される全体評価条件に達しない場合は本番採用しない
