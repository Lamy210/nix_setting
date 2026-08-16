## MODIFIED Requirements

### Requirement: nix repair / uninstall の昇格実行

GUI SHALL は `schneeforge nix repair` / `schneeforge nix uninstall` を GUI process 内で直接実行せず、CLI sidecar を昇格実行する (apply 系と同一の仕組み)。

#### Scenario: wizard から repair を実行する

- **WHEN** NixStatus が `Degraded` または `Broken` の状態でユーザーが wizard の「修復を試みる」を選択する
- **THEN** CLI sidecar (`schneeforge nix repair`) が管理者権限で実行される
- **AND** 結果 (実行した action または案内文案) が表示され、再確認へ戻れる

#### Scenario: repair は確認 dialog なしで実行できる

- **WHEN** ユーザーが wizard の「修復を試みる」を選択する
- **THEN** repair は非破壊の状態 (Healthy / Missing / 案内のみ) を含むため確認なしで実行する
- **AND** 唯一の破壊操作 (stale ownership record 削除) の内容は CLI 側の dry-run 同様の案内を含む

#### Scenario: Ready 画面から uninstall を確認付きで実行する

- **WHEN** ユーザーが Ready 画面の「Nix を削除」ボタンを押す
- **THEN** 確認 dialog (Nix と `/nix` 配下が削除される旨) を表示する
- **AND** 確認後にのみ CLI sidecar (`schneeforge nix uninstall`) が管理者権限で実行される
- **AND** `--force` は付与しない (ownership record 無しの uninstall は CLI の明示指定に限定)

#### Scenario: uninstall の確認をキャンセルする

- **WHEN** ユーザーが確認 dialog でキャンセルする
- **THEN** 何も実行せず元の画面に戻る

#### Scenario: repair / uninstall の失敗は CLI fallback 案内を出す

- **WHEN** 昇格が拒否された、または CLI が非 zero exit で失敗した
- **THEN** エラーと stdout/stderr の末尾を表示する
- **AND** CLI (`sudo schneeforge nix repair` / `sudo schneeforge nix uninstall`) での実行案内を表示する

#### Scenario: repair 実行後の状態は get_status に反映される

- **WHEN** repair の実行が完了する
- **THEN** frontend は `get_status` を呼び直し `nix_status` が更新される (例: `Broken` → `Missing`)
