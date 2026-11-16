# UIビジュアルリグレッションテスト

## 対象と保証範囲

UI変更PRでは、PRベースコミットとHEADを同じLinux Chromiumで描画して画像を比較します。比較結果はGitHub Actions artifactsに保存されます。

- 対象画面: dashboard、first-run、初期セットアップ6ステップ（ソース選択／マシン情報／プラン／適用確認／適用結果／検証結果）、nix-confirmation、nix-install-success、nix-install-failure、apply-result、verify-result、verify-failure、app-updater、app-update-result、scan-result、scan-failure、plan-result、rollback-result、source-update-result、upgrade-result、profile-selected、profile-cleared（800×600／1100×850の2サイズ）
- 出力: before.png、after.png、diff.png（変更領域を赤色表示）、results.json（`pixel_threshold`含む）、screenshots-manifest.json（各PNGのピクセル寸法・バイト数・SHA256）
- すべての定義済みシナリオを画面全体（Playwright `full_page=True`）で撮影し、**差分が0であっても変更前・変更後・差分PNGを必ず保存**します。現行は24シナリオ×2 viewport=48組、計144枚です。
- 画像の欠落・空ファイル・破損PNG・viewportより小さい画像・シナリオ欠落を検出したらCIを失敗させます。撮影が完了していない場合、成功したという証跡を残しません。
- この48組は定義済み画面状態の網羅であり、全てのネイティブ画面・モーダル・OS環境を網羅した保証ではありません。
- Job Summaryに差分ピクセル数・比率・RGB差分閾値とartifactリンクを表示します。
- RGB各チャンネルの差分閾値は既定で `0`（厳密比較）。`--pixel-threshold 0..255` で調整できます。画像比較の差分は手動レビュー対象でありCI失敗条件にはしません。
- UI操作テストでApply/Verify、空・不正なVerify結果の失敗扱い、IPC例外・失敗・再試行、Nix導入の成功と失敗、削除キャンセル、アプリ更新の進捗と再起動キャンセルを検証します。追加のブラウザー操作シナリオではScan（成功・失敗）、Plan、Rollback（成功・IPC拒否）、ソース更新、checkoutモードのUpgrade、プロファイル選択・既定復帰を撮影し、対応するIPC呼出しと操作後の表示も検証します。UI上の取得進捗はモックであり、実ダウンロードの証拠ではありません。
- **差分そのものは失敗にしません。** 意図的なデザイン変更も検出されるため、レビュー時に画像を確認します。画面取得失敗・操作テスト失敗はCI失敗です。
- PNGはコミットせずCI artifactsのみに保存します。既存の画像混入検査との衝突を防ぎます。

## スコープと安全性

このテストはTauri IPCの固定fixtureを使うブラウザーUI検証です。Tauriネイティブウィンドウ、管理者権限ダイアログ、実ダウンロード、Nixインストール、Home Managerの実適用を検証したものではありません。

Playwright 1.64.1／Pillow 13.0.0を固定し、同じジョブで両コミットの画面を撮ります。画面サイズ、表示言語、フォント環境、モック応答を固定し、127.0.0.1以外への通信を拒否します。GitHub-hosted runnerと `contents:read` 権限だけを使います。`check.yml`から常時呼び出す再利用可能ワークフローであり、個別ワークフローのpathフィルターでCIの必須判定を迂回しません。

実HTTPダウンロードの障害注入とLinux実適用E2Eは別のCIジョブで実施します。Linux x86_64の使い捨てGitHub-hosted runnerでは、SchneeForge CLIによる設定A→B→AのHome Manager世代・管理ファイル復旧を別E2Eで検証します。最新HEADのCIが成功した場合のみ実証完了と判断します。一方、TauriネイティブE2E、macOS/WSL2の実インストール・実適用、リリース配布環境の復旧は依然として未検証です。実機受入の完了とは区別してください。
