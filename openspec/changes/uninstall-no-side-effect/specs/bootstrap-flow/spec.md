## ADDED Requirements

### Requirement: uninstall は副作用を持たない
uninstall コマンド SHALL は削除レベルと手順を表示するのみで、state や設定を変更しない。

#### Scenario: uninstall を実行しても state が残る
- **WHEN** ユーザーが uninstall コマンドを実行する
- **THEN** 削除レベルと手順が表示される
- **AND** state ファイルは削除されない
