# Docker開発サンドボックス 仕様書

## 目的

Nix flake + Home Manager 設定のビルド検証を macOS 環境に依存せず実行可能にする。
CI でも利用できる Docker ベースの開発サンドボックスを構築する。

## 要件

- Dockerfile で Nix (flakes有効) が使えるコンテナを作成
- `docker compose` で `nix flake check` / `nix develop` を簡単に実行できる
- コンテナ内で `home.nix` のビルド検証が可能（aarch64-darwin のクロスビルドは不可だが、構文・型チェックは可）
- CI (`.github/workflows/check.yml`) でも Docker ベースの検証を追加

## 管理対象

| 分類 | 内容 |
|------|------|
| Dockerfile | Nix インストール + flakes 有効化 + 作業ディレクトリ設定 |
| docker-compose.yml | ビルド・チェック・シェル起動のサービス定義 |
| CI | `check.yml` に Docker 検証ジョブ追加 |

## 設計方針

- ベースイメージ: `nixos/nix` または `alpine` + Nix installer
- コンテナ内では `x86_64-linux` として動作（`system` の差は flake check で吸収）
- `docker compose run check` で `nix flake check` を実行
- `docker compose run dev` で `nix develop` を起動
- `.env` 的なボリュームマウントは不要（リポジトリを丸ごとマウント）

## ファイル構成

```
Dockerfile
docker-compose.yml
```

## 将来拡張

- multi-arch build による aarch64 クロス検証
- Nix store のキャッシュボリューム
