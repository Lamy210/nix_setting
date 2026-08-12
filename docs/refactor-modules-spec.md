# モジュール分割リファクタリング 仕様書

## 目的

- `home.nix` (278行) を責務別モジュールに分割し、保守性を向上
- `flake.nix` のハードコード値を `user-options.nix` に集約
- devShells を独立ファイルに分離

## 参考

nix-darwin + Home Manager のモジュール分割構成を持つ既存リポジトリを参考に、
本リポジトリでも同様の分割方針を採用する。

## 管理対象

| 分類 | 内容 |
|------|------|
| home.nix | → `modules/default.nix` (エントリポイント) + 以下に分割 |
| home.packages | → `modules/packages.nix` |
| zsh / shell | → `modules/shell.nix` (zshInitExtra, programs.zsh, aliases, completions) |
| programs | → `modules/programs.nix` (starship, fzf, direnv, zoxide, atuin, bat, tmux) |
| dotfiles | → `modules/dotfiles.nix` (home.file, xdg.configFile) |
| flake.nix | `user-options.nix` から system/username/homeDirectory を読み込み |
| devShells | → `modules/devshells.nix` に切り出し |

## 変更前後の構成比較

```
変更前:
  flake.nix         # 全 devShell + homeConfigurations
  home.nix          # 278行、1ファイルに全集中
  user-options.nix  # 定義はあるが使われていない

変更後:
  flake.nix          # 簡潔化、user-options.nix を import
  home.nix           # modules/default.nix を import するだけのエントリポイント
  modules/
    default.nix      # エントリポイント (旧 home.nix の { pkgs, ... }: 本体)
    packages.nix     # home.packages = with pkgs; [...]
    shell.nix        # programs.zsh + zshInitExtra (+ let binding)
    programs.nix     # programs.starship/fzf/direnv/zoxide/atuin/bat/tmux
    dotfiles.nix     # home.file / xdg.configFile
    devshells.nix    # devShells 定義 (flake.nix から移動)
```

## 設計方針

- `user-options.nix` の出力を `flake.nix` で import して利用（ハードコード排除）
- 各モジュールは `{ pkgs, ... }: { ... }` の形式
- `shell.nix` 内の `let zshInitExtra = ...` バインディングはそのまま維持
- 既存の動作を一切変えない（リファクタリングのみ）

## 将来拡張

- `.config/openspec/` の代わりに `nix-darwin` で管理
