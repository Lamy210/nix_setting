# 実機 switch 手順

CI は build まで検証するが、実際の `switch`（適用）は実機で行う。通常は `schneeforge plan` → `schneeforge apply` を使用し、復旧は `schneeforge rollback` → `schneeforge verify` を使用する。CLI は実機用 machine/profile input を解決する。以下は実機用 input を設定済みの開発者向け直接操作であり、`defaults/machine.nix` の placeholder のまま適用しないこと。

## macOS

```bash
# 1. 設定変更を適用 (nix-darwin + home-manager 一括)
nh darwin switch . -H darwin-aarch64

# または素の nix-darwin
nix run --inputs-from . nix-darwin#darwin-rebuild -- switch --flake .#darwin-aarch64

# 2. ロールバック (1世代戻す)
darwin-rebuild --rollback

# 3. generation 確認
darwin-rebuild --list-generations
```

## Linux

```bash
# 1. 適用
nh home switch . -c linux

# 2. ロールバック
home-manager generations
nix run --inputs-from . home-manager#home-manager -- switch --rollback

# 3. generation 確認
home-manager generations
```

## 検証手順

```bash
# switch 前に必ず build で確認
nix build .#darwinConfigurations.darwin-aarch64.system --no-link   # macOS
nix build .#homeConfigurations.linux.activationPackage --no-link # Linux

# 問題なければ switch
nh darwin switch . -H darwin-aarch64
```

## 注意

- `switch` は実環境を変更する。CI では build のみ
- Homebrew cask の初回適用は時間がかかる（`homebrew.onActivation` 設定に従う）
- WezTerm 設定は `Ctrl+Shift+R` で再読込
- バックアップは `~/hm-bak-*` に自動保存される
