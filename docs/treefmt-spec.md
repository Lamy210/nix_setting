# treefmt-nix + checks 分離 仕様書

## 目的

- `nix fmt` で nixfmt + shfmt + taplo を統合実行
- `nix flake check` から `checks.treefmt` で同一 formatter contract を検証
- lefthook も同じ `nix fmt` を呼ぶ

## 実装

`treefmt-nix` の `flakeModule` を flake-parts に import。
設定は `modules/flake-parts/default.nix` の `treefmt.config` に記述。

```nix
# modules/flake-parts/default.nix
imports = [
  inputs.treefmt-nix.flakeModule
  ...
];
perSystem = { ... }: {
  treefmt.config = {
    projectRootFile = "flake.nix";
    programs = {
      nixfmt.enable = true;
      shfmt.enable = true;
      taplo.enable = true;
    };
  };
};
```

`flakeModule` が自動的に以下を生成:
- `formatter` — `nix fmt` で呼ばれる
- `checks.treefmt` — `nix flake check` で整形検証

## 対象ファイル種別

| 種別 | formatter |
|------|-----------|
| `*.nix` | nixfmt |
| `*.sh` | shfmt |
| `*.toml` | taplo |

## 除外

- `flake.lock`
- `.gitignore`
- `.envrc`
- 画像ファイル (`*.jpg`, `*.png`, `*.gif`, `*.webp`)
