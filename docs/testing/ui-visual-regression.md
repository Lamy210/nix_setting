# UIビジュアルリグレッションテスト

## 対象と保証範囲

UI変更PRでは、PRベースコミットとHEADを同じLinux Chromiumで描画して画像を比較します。比較結果はGitHub Actions artifactsに保存されます。

- 対象画面: dashboard、first-run、nix-confirmation、nix-install-success、nix-install-failure、apply-result、verify-result、verify-failure、app-updater、app-update-result（800×600／1100×850の2サイズ）
- 出力: before.png、after.png、diff.png（変更領域を赤色表示）、results.json（`pixel_threshold`含む）、screenshots-manifest.json（各PNGのピクセル寸法・バイト数・SHA256）
- すべての定義済みシナリオを画面全体（Playwright `full_page=True`）で撮影し、**差分が0であっても変更前・変更後・差分PNGを必ず保存**します。現行は10シナリオ×2 viewport=20組、計60枚です。
- 画像の欠落・空ファイル・破損PNG・viewportより小さい画像・シナリオ欠落を検出したらCIを失敗させます。撮影が完了していない場合、成功したという証跡を残しません。
- この12組は定義済み画面状態の網羅であり、全てのネイティブ画面・モーダル・OS環境を網羅した保証ではありません。
- Job Summaryに差分ピクセル数・比率・RGB差分閾値とartifactリンクを表示します。
- RGB各チャンネルの差分閾値は既定で `0`（厳密比較）。`--pixel-threshold 0..255` で調整できます。画像比較の差分は手動レビュー対象でありCI失敗条件にはしません。
- UI操作テストでApply/Verify、空・不正なVerify結果の失敗扱い、IPC例外・失敗・再試行、Nix導入の成功と失敗、削除キャンセル、アプリ更新の進捗と再起動キャンセルを検証します。UI上の取得進捗はモックであり、実ダウンロードの証拠ではありません。
- **差分そのものは失敗にしません。** 意図的なデザイン変更も検出されるため、レビュー時に画像を確認します。画面取得失敗・操作テスト失敗はCI失敗です。
- PNGはコミットせずCI artifactsのみに保存します。既存の画像混入検査との衝突を防ぎます。

## スコープと安全性

このテストはTauri IPCの固定fixtureを使うブラウザーUI検証です。Tauriネイティブウィンドウ、管理者権限ダイアログ、実ダウンロード、Nixインストール、Home Managerの実適用を検証したものではありません。

Playwright 1.64.1／Pillow 13.0.0を固定し、同じジョブで両コミットの画面を撮ります。画面サイズ、表示言語、フォント環境、モック応答を固定し、127.0.0.1以外への通信を拒否します。GitHub-hosted runnerと `contents:read` 権限だけを使います。`check.yml`から常時呼び出す再利用可能ワークフローであり、個別ワークフローのpathフィルターでCIの必須判定を迂回しません。

実HTTPダウンロードの障害注入とLinux実適用E2Eは別のCIジョブで実施します。一方、TauriネイティブE2E、macOS/WSL2の実インストール・実適用、Home Manager世代ロールバックは未検証です。実機受入の完了とは区別してください。
