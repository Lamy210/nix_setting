# nix-darwin + Homebrew 統合 仕様書

## 目的

現在 `Brewfile` で手動管理している Homebrew cask を nix-darwin 経由で宣言的に管理する。
既存の nix-darwin + Homebrew 統合リポジトリの `nix-darwin/config/homebrew/` 方式を参考にする。

## 要件

- flake.nix に `darwinConfigurations` 出力を追加
- `nix-darwin/config/homebrew/` で cask を宣言的に管理
- 既存の `Brewfile` は旧管理方式として残す（移行後削除）
- 既存の Home Manager 設定と競合しないように統合
- `home-manager` は nix-darwin の module として組み込む

## 管理対象

| 分類 | 内容 |
|------|------|
| nix-darwin エントリ | `nix-darwin/default.nix` |
| Homebrew cask | `nix-darwin/config/homebrew/` (casks + taps) |
| Nix 設定 | `nix-darwin/config/nix-config.nix` |
| システム設定 | `nix-darwin/config/system.nix` (macOS defaults) |

## Brewfile → Nix 変換対応表

| Brewfile | nix-darwin homebrew.casks |
|----------|--------------------------|
| `font-plemol-jp-nf` | `"font-plemol-jp-nf"` |
| `font-jetbrains-mono-nerd-font` | `"font-jetbrains-mono-nerd-font"` |
| `wezterm` | `"wezterm"` |
| `visual-studio-code` | `"visual-studio-code"` |
| `loop` | `"loop"` |
| `flashspace` | `"flashspace"` |
| `tameo` (tap required) | tap は `homebrew.taps` で管理 |

## 設計方針

```
flake.nix
  darwinConfigurations.default = nix-darwin.lib.darwinSystem {
    system = "aarch64-darwin";
    modules = [
      ./nix-darwin/default.nix
      home-manager.darwinModules.home-manager
      {
        home-manager.users.${username} = import ./home.nix;
      }
    ];
  };
```

- `nix-darwin/default.nix` が以下の設定を import:
  - `./config/nix-config.nix` (nix settings)
  - `./config/system.nix` (macOS defaults — 最小限)
  - `./config/homebrew/default.nix` (casks + taps)
- 適用コマンド: `nix run nix-darwin -- switch --flake .`

## 将来拡張

- macOS defaults (Dock, Finder, Trackpad 等) の宣言的管理
- nix-darwin 経由での launchd サービス管理
- `homebrew.masApps` による Mac App Store アプリ管理
