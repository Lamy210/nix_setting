# hosts 分離 + Linux Home Manager + macOS ARM CI 仕様書

## 目的

- ホスト固有設定を `hosts/` に分離
- Linux用 Home Manager 環境を追加（`homeConfigurations.linux`）
- macOS ARM CI runner 追加

## 対象構成

```
hosts/
├── macbook-air/
│   └── default.nix     # nix-darwin + home-manager (macOS)
└── linux-generic/
    └── default.nix     # home-manager only (Linux)
```

## flake.nix 変更

```nix
homeConfigurations = {
  macbook-air = ...;   # macOS home-manager (既存のdefault)
  linux = ...;         # Linux home-manager (新規)
};
darwinConfigurations.macbook-air = ...;  # macOS nix-darwin (既存のdefault)
```

## macOS ARM CI

```yaml
macos-check:
  runs-on: macos-latest
  steps:
    - uses: cachix/install-nix-action@v30
    - run: nix flake check
    - run: nix build .#homeConfigurations.macbook-air.activationPackage
```

## 設計方針

- `hosts/macbook-air/default.nix` = 現行の `home.nix` 相当 + macOS 固有
- `hosts/linux-generic/default.nix` = Linux 用の最小構成（macOS 専用パッケージ除外）
- `modules/packages.nix` は共通部分のみに。OS 依存パッケージは `hosts/` 側に移動
- `user-options/options.nix` で username/homeDirectory/OS情報を管理

## 将来拡張

- VPS 用ホスト追加
- NixOS ホスト追加
