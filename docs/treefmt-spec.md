# treefmt-nix + checks 分離 仕様書

## 目的

- `nix fmt` で nixfmt + shfmt + taplo を統合実行
- `nix flake check` から `checks.formatting` で同一 formatter contract を検証
- lefthook も同じ formatter を参照

## 管理対象

| 分類 | 内容 |
|------|------|
| formatter | `modules/checks/formatting.nix` (treefmt-nix) |
| checks | `modules/checks/formatting.nix` → `checks.formatting` |
| lefthook | `nix fmt` を呼ぶだけに簡略化 |
| flake input | `treefmt-nix` 追加 |

## 対象ファイル種別

| 種別 | formatter |
|------|-----------|
| `*.nix` | nixfmt |
| `*.sh` | shfmt |
| `*.toml` | taplo |
| `*.yml` / `*.yaml` | (treefmt-nix に yaml formatter なし、prettier 等が必要だが今回はスキップ) |

## 設計方針

```
flake.nix
  inputs.treefmt-nix 追加
  perSystem:
    formatter = treefmt-nix の config.build.wrapper
    checks.formatting = treefmt-nix の config.build.check

modules/checks/
  formatting.nix   # treefmt-nix 設定
```

- `nix fmt` → 全ファイル整形
- `nix flake check` → `checks.formatting` で整形済みか検証
- lefthook → `nix fmt` を呼ぶ

## 将来拡張

- yaml/json formatter 追加 (prettier or dprint)
- lua formatter (stylua)
