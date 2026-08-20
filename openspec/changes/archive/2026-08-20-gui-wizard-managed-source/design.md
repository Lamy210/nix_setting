# Design: GUI wizard の managed source 対応

## D1: source 初期化状態の判定 (Diagnostics 拡張)

`Diagnostics` に `managed_source: Option<ManagedSourceSummary>` を追加する:

```rust
#[derive(Serialize)]
pub struct ManagedSourceSummary {
    pub tag: String,
    pub channel: String,
    pub flake_ref: String,
}
```

- `StateStore` に managed Release source がある場合 `Some`、それ以外
  (checkout 表現 / 未初期化) は `None`
- frontend の「source 初期化済み」判定は `repo_exists || managed_source != null`
- これにより managed で初期化した machine (repo 無し) が setup に回らなくなる

採用しない案: `repo_exists` の意味を変える (既存 frontend / test が
checkout の存在確認として広く参照しており、意味の多重化は危険)。

## D2: stepRepo の再構成 (source 選択 step)

wizard の step 1 (旧 stepRepo) を「configuration source」step にする:

- **managed (default・推奨)**: `run_source_init` を呼ぶ。tag 指定は UI から
  出さず CLI (`source init --tag`) と同じ既定動作 (channel stable の最新 tag
  を `git ls-remote` + ReleaseMetadata で解決)。git は preflight の tool
  解決に乗る
- **git clone (fork / 開発者向け)**: 既存 `run_clone_repo` をそのまま残す
  (URL 入力も従来通り)

`run_source_init` command は core `source_init(repo, store, git, channel, tag)`
を `spawn_blocking` で呼び、`CommandOutput` を返す (既存 command pattern と
同一)。成功時の state 保存は core 側が行う。

macOS の初回適用は darwin-rebuild が内部で sudo を要求するため、apply は
従来通り escalated CLI sidecar (`apply`) を使う。escalation の
`NIX_SETTING_DIR` 渡しは変更しない (managed state では CLI が repo path を
build に使わないため、存在しない path が渡っていても無害)。

## D3: Managed Nix install の repo gate 削除

stepPrereq の「まず repository の clone が必要です」gate と
`nix_prepare_plan_blocking` の `bootstrap-manifest.toml` 存在 check を
削除する。根拠:

- escalated 先の CLI sidecar は `ManagedNix::load_prefer_repo` (repo file
  優先 → embedded fallback) で manifest を解決するため、repo 無しで
  embedded manifest が使われる (`switch-install-sh-to-managed-source` D1)
- release unit: DMG 同梱の sidecar は app と同一 source tree から build
  されるため、embedded manifest は「その app 自身の release の manifest」
  と一致する

## D4: wizard 以降の step は不変

stepUser (machine facts 表示) / stepPlan / stepConfirm / stepApply /
stepVerify は managed source でもそのまま動く:

- plan / apply は core `effective_ref` が flake ref に解決 (§7)
- verify は managed 向け check に切替済み (`repository_check`)
- machine input は state dir へ書かれ repo 非依存 (v2 MachineFacts)

## D5: 互換性

- 既存 checkout user: `repo_exists=true` → 従来通り stepRepo で clone skip
  (managed 選択 UI は表示されるが、既定で「登録済み」表示にする)
- 途中中断への冪等性: `run_source_init` は `source init` と同じ再実行
  挙動 (同一 tag なら移行表示、別 tag なら上書き)

## D6: scope 外

- Dashboard からの update 実行 (本体自己 update, Phase E)
- checkout 表現 Release の update 経路の廃止
- wizard の UI デザニューアル (既存素朴な DOM 構造を維持)
