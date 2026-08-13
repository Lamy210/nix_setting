# SchneeForge Desktop (Tauri 2)

デスクトップ GUI。`schneeforge-core` を呼ぶ薄い front-end。

## 構成

```
apps/desktop/
├── src-tauri/        # Rust backend (独立 Cargo workspace)
│   ├── src/lib.rs    # Tauri commands (get_status/run_scan/run_apply/...)
│   ├── tauri.conf.json
│   └── icons/
└── dist/
    └── index.html    # ダークテーマ UI
```

## 開発

```bash
nix develop                     # cargo-tauri を含む devShell
cd apps/desktop/src-tauri
cargo tauri dev                 # 開発用に起動 (ホットリロード)
```

## ビルド

```bash
cargo tauri build               # release ビルド (target/release/ に出力)
```

## 注意

- `.app` バンドル化・DMG 化・署名・notarization は未実装（Phase E）
- 独立 Cargo workspace のため、CLI ビルドは Tauri 依存に巻き込まれない
- `gen/` は自動生成（gitignore 対象）

## 既知の脆弱性

- `glib` (0.18.x): `VariantStrIter` の unsoundness (medium, Linux のみ)。
  パッチ版は glib 0.20+ だが、tauri の gtk-rs 依存が 0.18 系に固定しており現状修正不可。
  tauri が gtk-rs を更新したら解消される。macOS には影響なし。
