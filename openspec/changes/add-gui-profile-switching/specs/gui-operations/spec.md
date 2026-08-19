## ADDED Requirements

### Requirement: GUI からの profile 切替

desktop SHALL は async command `get_profiles` / `set_profile(name)` /
`clear_profile` を提供する。`set_profile` は manifest の
`profiles.available` 検証を行い、検証を通った場合のみ state へ保存する
(fail-closed)。manifest 取得を伴うため UI thread を占有しない。

#### Scenario: profile 一覧の取得

- **WHEN** frontend が `get_profiles` を invoke する
- **THEN** manifest の available / default と state の selected を持つ `ProfileList` が返る

#### Scenario: 不正 profile の拒否

- **WHEN** available に無い名前で `set_profile` を invoke する
- **THEN** command は error を返し、state の選択は変更されない

#### Scenario: 選択の解除

- **WHEN** `clear_profile` を invoke する
- **THEN** state の選択が解除され、以降の解決は manifest default を使う

### Requirement: Dashboard での profile 切替 UI

frontend SHALL は Dashboard に profile 切替 UI (manifest available からの
選択 + 適用) を表示する。切替は state のみを変更するため、**次回の apply
から反映される**旨を UI に表示する。

#### Scenario: 切替の反映案内

- **WHEN** `set_profile` が成功する
- **THEN** UI は次回の apply から反映される旨を表示し、status / dashboard の表示を更新する

#### Scenario: manifest が解決できない場合

- **WHEN** `get_profiles` が manifest 不在などの error を返す
- **THEN** 切替 UI は使用できない旨を表示し、Dashboard の他の表示は通常通り行う

### Requirement: frontend と backend の契約一致 (profile 切替)

`ProfileList` の serialize key (available / default / selected) への
frontend 参照と、切替 UI の DOM id SHALL は backend の定義と一致する。
desktop の unit test が serialize key + frontend 参照 + DOM id の対応を
検証する。

#### Scenario: key 参照の regression 検証

- **WHEN** desktop の test suite が実行される
- **THEN** `ProfileList` の serialize key と `main.js` の当該 key 参照・切替 UI の DOM id が検証される
