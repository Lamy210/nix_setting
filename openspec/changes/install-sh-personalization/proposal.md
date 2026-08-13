## Why

install.sh / bootstrap.sh は repository を clone した後、committed された `config.toml`（`username = "lamy210"`）をそのまま適用する。第三者がこのインストーラを使うと、lamy210 の username で Home Manager が構築され、誤った homeDirectory が設定される。GUI の First Run Wizard は OS から username を取得して config を生成するが、シェル（install.sh / bootstrap.sh）経路にはその個人化が無い。

## What Changes

- bootstrap.sh が apply 前に OS の実行ユーザー名（`whoami`）から `config.toml` を生成し、committed された username をそのまま適用しない
- install.sh（clone → bootstrap.sh を呼ぶ）は bootstrap.sh 経由で個人化される

## Capabilities

### New Capabilities

（なし）

### Modified Capabilities

- `bootstrap-flow`: install 時の username 個人化 requirement を追加

## Impact

- `bootstrap.sh`: apply 前に config.toml を OS username で生成
- テスト: shellcheck / bash -n
