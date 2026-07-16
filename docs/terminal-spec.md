# Terminal-Nix 詳細仕様書

## 基本方針

- Nix + Home Manager で dotfiles と CLI ツールを宣言管理
- WezTerm + zsh + Starship を基本構成
- public repo には秘密情報・背景画像を含めない
- ユーザー名やホームディレクトリは `bootstrap.sh` で自動生成

## 管理対象

| 分類 | 内容 |
|------|------|
| Terminal | WezTerm (Lua設定) |
| Shell | zsh |
| Prompt | Starship (TOML) |
| CLI | eza, bat, fd, ripgrep, fzf, zoxide, atuin |
| Git UX | lazygit, delta, gh, ghq |
| Dev | yazi, btop, dust, duf, direnv, mise, just, tmux |

## 将来拡張

- nix-darwin 統合 (Homebrew cask / macOS defaults)
- fish / nixvim / devShells
- CI (GitHub Actions で secret scan / flake check)
