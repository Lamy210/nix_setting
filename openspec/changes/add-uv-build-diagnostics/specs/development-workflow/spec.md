## ADDED Requirements

### Requirement: Independent Python flake build diagnostics

開発者向けuvビルド診断はroot Python devShellと配布可能な独立Python templateの対象system、独立lock、workload SHA、harness SHA、実際に選択されたuv derivationとvendor/input derivation、runtime executable/versionの関係を追跡しなければならない (MUST)。同一versionやsource hashだけでderivationやキャッシュ再利用の同一性を保証してはならない (MUST NOT)。初期診断は既存lock・設定・信頼境界を変更してはならない (MUST NOT)。

#### Scenario: 同一uv versionでderivationが異なる
- **WHEN** rootと独立templateが同じversionのuvを選択するがderivationまたは入力が異なる
- **THEN** 各derivationと入力差分を別個の証拠として記録する
- **AND** 同一ビルド・同一キャッシュ利用とは判定しない

#### Scenario: 選択されたuvを一意に特定できない
- **WHEN** 対象入力が欠落・不正・曖昧である、または実体versionが期待と異なる
- **THEN** 候補欠損・曖昧・証拠不足はunknown、入力形式不正・期待version不一致・契約違反はfailedと理由を記録し、全体の終了コードを非0とする
- **AND** lockや診断対象のコードを自動修正しない

#### Scenario: 配布されるPython templateを診断する
- **WHEN** root repositoryから独立したtemplateを検証する
- **THEN** 独立lockと配布ファイルだけを使用した検証を行う
- **AND** 親repoへの暗黙参照を必須にしない

#### Scenario: runtime検証が許可されていない
- **WHEN** evaluationのみを実施し、runtimeの実行許可または証拠がない
- **THEN** evaluationの結果とruntimeのnot-run / unknownを別記録する
- **AND** 必須証拠不足のまま全体をverifiedとして終了コード0を返さない

### Requirement: Isolated uv build comparison evidence

uv重複ビルドPoCは評価・realization・runtimeを区別し、変更前のworkloadと明示された候補差分を隔離環境で比較しなければならない (MUST)。各比較条件のcold/warm、成功件数、失敗・キャンセル・skip、runner実行時間、実費またはunknownを記録し、各対応条件で両対象jobが成功した比較可能なペア10件未満または比較可能性未確認なら探索に限定しなければならない (MUST)。公開する証拠から秘密情報を除外しなければならない (MUST)。

#### Scenario: 比較サンプルを十分に取得できない
- **WHEN** 承認済み予算に達した、または対応条件の成功サンプルが10件未満である
- **THEN** 比較を探索結果として記録し、必要に応じてDeferとする
- **AND** 性能目標達成・実費削減を宣言しない

#### Scenario: 異なる実行方法の結果が得られる
- **WHEN** candidateがsource-built、substituted、already-present等の異なる方法で検証される
- **THEN** 実行方法と各stageのoutcomeを分けて記録する
- **AND** evaluationのみの成功をrealizationまたはruntime成功としない

#### Scenario: 診断出力に機密値が含まれる
- **WHEN** derivationの入力や環境値に秘密情報または個人識別情報が存在する
- **THEN** 許可された証拠フィールドだけを抽出し、機密値をマスクまたは除外する
- **AND** 生の機密情報をPRや公開artifactに出力しない

#### Scenario: 比較対象の実行条件が一致しない
- **WHEN** cache状態がunknownである、job定義または検証範囲が異なる、あるいはローカル予備計測しか取得できない
- **THEN** 正式なCI job elapsed比較から除外し理由と探索結果を保持する
- **AND** 検証を省略した短時間runを性能改善として扱わない

### Requirement: uv build optimization candidate approval

uvビルド候補の採用判断は独立template互換性、既存security backport、必須CIの非回帰、および比較可能なペアの対象2ジョブrunner elapsed合計中央値20%以上削減を必要とする (MUST)。baseline中央値0またはunknownのとき改善率はunknownとしなければならない (MUST)。x86_64-linuxの初期比較でのGoは後続changeの提案に限定し、未検証systemの恒久採用を承認してはならない (MUST NOT)。この個別目標の達成をCI全体の既存critical-path目標達成と同一視してはならない (MUST NOT)。PoCは恒久的なpin/cache/trust/workflow変更を自動適用してはならず、別changeで承認・実装しなければならない (MUST)。

#### Scenario: 対象2ジョブの改善候補が得られる
- **WHEN** 比較可能なbaselineとcandidateの対応するroot/template jobのelapsed合計中央値が20%以上減少する
- **THEN** 既存security・独立Flake・検証被覆を確認して別changeの提案可否を判断する
- **AND** CI全体の高速化完了やキャッシュ採用済みとは宣言しない

#### Scenario: 性能は改善するが安全性が低下する
- **WHEN** candidateがsecurity overrideの喪失、独立templateの破損、必須CIの退行のいずれかを起こす
- **THEN** No-Goまたは修正可能性を明記したDeferとする
- **AND** 元のpinと構成を維持し、採用しない
