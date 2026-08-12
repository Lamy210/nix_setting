# 実機 switch 手順

CI は build まで検証するが、実際の `switch`（適用）は実機で行う。

## macOS

```bash
# 1. 設定変更を適用 (nix-darwin + home-manager 一括)
nh switch .#macbook-air

# または素の nix-darwin
nix run nix-darwin -- switch --flake .#macbook-air

# 2. ロールバック (1世代戻す)
darwin-rebuild --rollback
# または nix-darwin の generation を指定
darwin-rebuild switch --flake .#macbook-air --generation <n>

# 3. generation 確認
darwin-rebuild --list-generations
```

## Linux

```bash
# 1. 適用
nh home switch .#linux

# 2. ロールバック
home-manager generations
/home/user/.nix-profile/bin/home-manager switch --flake .#linux --rollback

# 3. generation 確認
home-manager generations
```

## 検証手順

```bash
# switch 前に必ず build で確認
nix build .#darwinConfigurations.macbook-air.system --no-link   # macOS
nix build .#homeConfigurations.linux.activationPackage --no-link # Linux

# 問題なければ switch
nh switch .#macbook-air
```

## 注意

- `switch` は実環境を変更する。CI では build のみ
- Homebrew cask の初回適用は時間がかかる（`homebrew.onActivation` 設定に従う）
- WezTerm 設定は `Ctrl+Shift+R` で再読込
- バックアップは `~/hm-bak-*` に自動保存される
