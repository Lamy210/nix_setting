# CI Green + 検証ギャップ修正 仕様書

## 目的

- 現在 red の CI を完全 green に戻す
- 検証の穴（テンプレート未CI、Docker pin、arch判定、SBOM保存、SHA pin）を塞ぐ

## P0: CI green

### 1. weekly.yml SC2046

`sbomnix` の `$(nix path-info ...)` を変数に抽出してクォート。

```bash
target="$(nix path-info .#homeConfigurations.linux.activationPackage)"
nix run nixpkgs#sbomnix -- "$target" --output sbom.cdx.json
```

### 2. statix empty pattern

引数を未使用のモジュールの `{ ... }:` → `_:` に変更。

対象:
- `nix-darwin/config/homebrew/default.nix`
- `nix-darwin/config/nix-config.nix`
- `nix-darwin/config/system.nix`
- `nix-darwin/default.nix`
- `hosts/macbook-air/default.nix`
- `hosts/linux-generic/default.nix`
- `modules/dotfiles.nix`
- `modules/programs.nix`
- `modules/flake-parts/default.nix` (perSystem)

**注意**: `{ }:` には戻さない（Home Manager module は追加引数を受けるため）。

### 3. statix repeated keys

`modules/programs.nix` を `programs = { ... }` 構造に統合。

## P1: 検証ギャップ

### 4. テンプレート CI matrix

`template-check` を matrix 化:

```yaml
strategy:
  matrix:
    template: [devenv, node, python, rust]
```

各テンプレートで `nix flake metadata` + `nix flake lock` を検証。

### 5. Docker pin

`FROM nixos/nix:latest` → digest 固定。

### 6. bootstrap arch 判定

```
Darwin arm64/aarch64 → macbook-air
Darwin その他        → unsupported (Intel Mac を macbook-air に落とさない)
Linux x86_64        → linux
Linux aarch64       → linux-arm
Linux その他        → unsupported
```

### 7. SBOM artifact

生成した SBOM を `actions/upload-artifact` で保存。

### 8. update.yml SHA pin

`update.yml` の `DeterminateSystems/nix-installer-action@v16` と `update-flake-lock@v24` を full SHA に固定。
