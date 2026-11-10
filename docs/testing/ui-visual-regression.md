# UIビジュアルリグレッションテスト

## 対象と保証範囲

UI変更PRでは、PRベースコミットとHEADを同じLinux Chromiumで描画して画像を比較します。比較結果はGitHub Actions artifactsに保存されます。

- 対象画面: dashboard、first-run、nix-confirmation、apply-result、verify-result、app-updater（800×600／1100×850の2サイズ）
- 出力: before.png、after.png、diff.png（変更領域を赤色表示）、results.json
- Job Summaryに差分ピクセル数・比率とartifactリンクを表示します。
- UI操作テストでApply/Verify、IPC例外・失敗・再試行、Nix導入の成功と失敗、削除キャンセル、アプリ更新の進捗と再起動キャンセルを検証します。UI上の取得進捗はモックであり、実ダウンロードの証拠ではありません。
- **差分そのものは失敗にしません。** 意図的なデザイン変更も検出されるため、レビュー時に画像を確認します。画面取得失敗・操作テスト失敗はCI失敗です。
- PNGはコミットせずCI artifactsのみに保存します。既存の画像混入検査との衝突を防ぎます。

## スコープと安全性

このテストはTauri IPCの固定fixtureを使うブラウザーUI検証です。Tauriネイティブウィンドウ、管理者権限ダイアログ、実ダウンロード、Nixインストール、Home Managerの実適用を検証したものではありません。

Playwright 1.64.1／Pillow 13.0.0を固定し、同じジョブで両コミットの画面を撮ります。画面サイズ、表示言語、フォント環境、モック応答を固定し、127.0.0.1以外への通信を拒否します。GitHub-hosted runner、pull_request、contents:readの権限だけを使います。

次の開発段階では、HTTP障害注入、Linux実適用E2E、TauriネイティブE2E、macOS/WSL2実機受入を別ジョブで追加します。
