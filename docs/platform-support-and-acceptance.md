# SchneeForge: OS・CPU対応とインストール受入契約

> 2026-11-10 初版。仕様と試験計画であり、未実行・未合格の環境の動作を保証しない。

## 目的
インストール前の静的/動的検証、インストール後の機能確認、更新・ロールバック、
アンインストールと再インストールの安全性までを、OS・CPUごとに実測する。

## 用語
- **設計対象**: 実装・試験を計画しているだけ。
- **コンパイル検証済み**: 対象OS/CPUのネイティブrunnerで実際のCLI/launcherをbuild。
- **評価/ビルド検証済み**: Nix flake eval、home-manager/nix-darwin derivation buildも実測。
- **インストール検証済み**: 新規導入→doctor/verify→再適用→rollback→uninstall/reinstallの実測。
- **配布受入済み**: タグに固定した正規リリースアセットをクリーン実機で検証。
  どの段階も下位段階の成功と取り違えない。

## 初期プラットフォーム・マトリクス

| ターゲット | Nix system | 既存の証跡 | 次の未達ゲート |
| --- | --- | --- | --- |
| Apple Silicon Mac (M1/M2/M3/M4/mini) | aarch64-darwin | macos-15/26上で既存のNix/CLI/DMG CI | Finder / Nix新規導入 / 本物のM4 mini実機 |
| Intel Mac | x86_64-darwin | Nix本体/nix-darwinは対象。SchneeForgeのflake/output/配布は未対応 | ネイティブCLI canary、専用Nix systemと導入providerの検証 |
| Ubuntu/Debian x64 | x86_64-linux | Linux CI・musl CLI・Docker E2E | 実機/VMのhost installと再導入 |
| Linux ARM64 | aarch64-linux | flakeにhomeConfigurations.linux-armが存在。評価CIあり | ARM64ネイティブbuildと導入受入 |
| Windows x64 + Ubuntu WSL2 | Windows native launcher + x86_64-linux | transport-only helper canary（本物のNixは未導入） | WSL内のLinux CLI+Nix provider E2E |
| Windows ARM64 + WSL2 | Windows native launcher + aarch64-linux | 未検証 | ネイティブlauncher + ARM64 WSL2実機E2E |
| Fedora/RHEL系、AlmaLinux、Rocky Linux | x86_64-linux / aarch64-linux | Linux設定の転用可能性はあるが未検証 | SELinux・systemd・ホームディレクトリ・権限・パッケージ差分 |

注：GitHubのmacOS arm64標準runnerはM1。M4での動作保証やIntel Mac向けNix
インストールproviderの確定を意味しない。Mac miniは別Nix systemではなく同じ
aarch64-darwinで、必要ならハードウェア依存差分を実機テストする。

## 一次資料
- Nix対応プラットフォーム: https://nix.dev/manual/nix/2.34/installation/supported-platforms.html
- NixOS/nix-installer: https://github.com/NixOS/nix-installer
- nix-darwinのIntel/Apple Silicon設定: https://github.com/nix-darwin/nix-darwin
- GitHub runnerのOS/アーキ: https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- WSL2 systemd: https://learn.microsoft.com/en-us/windows/wsl/systemd

## 検証レイヤー

### L0: Commit/PRでの静的ゲート
- Nix: nixfmt、statix、deadnix、flake eval、OpenSpec validate、Nix入力lock検査
- Rust: fmt、clippy、cargo test --locked、両方のCargo.lock整合性
- CI/シェル: actionlint、shellcheck、Bats、最小権限・Action SHAの確認
- セキュリティ: secret scan、CodeQL、依存脆弱性チェック（導入前に結果を吟味）
- ダウンロード: hash、SLSA/provenance、悪意あるミラー・404・ネットワーク失敗のテスト
- ライセンス/依存更新: メジャー更新・MSRV・相互依存・複数lockfileを別途レビュー

### L1: インストール前の動的ゲート
- 対象OS/CPUのネイティブrunnerでCLIをビルドして実行
- Nixの評価と必要なactivationPackageのビルド（`switch`は行わない）
- リリースで配布するCLI/DMGを本番scriptと同じ経路でbuild・起動
- 読み取り専用preflight：OS/arch、systemd/WSL2、既存Nixの所有権・daemon、
  書き込み可能域、容量、署名・ハッシュ、必要な権限、依存ランタイムを確認
- UNKNOWNはPASS扱いしない。ログに根拠と失敗を記録し、副作用を起こさない。

### L2: 隔離環境へのインストール
- fresh Linux/WSL2 VM・使い捨てmacOS実機/runnerでのみホストを変更
- `install`→`nix doctor`→`plan`→`apply`→`verify`
- 同じマニフェストの再適用、予期しない既存Nixに対するfail-closed動作
- 中断/破損/ディスク不足/ネットワーク切断の障害注入
- 変更前後のプロセス・daemon・mount・receipt/所有権・世代を比較

### L3: リリース・アップデート受入
- 正規タグで配布されたアセットのチェックサム・provenance照合
- インストール直後のCLI/GUI smoke、起動経路、OS別Gatekeeper/実行権限
- 世代バックアップ→アップデート→verify→rollback→uninstall/reinstall
- macOS Finder GUI / `/dev/tty` / nix-darwinの完全なA–Jチェックは別途残す
- `v0.2.0-rc.8`はNodeセキュリティ等の既存P0が閉じるまで公開しない

## CIの段階的拡張

`.github/workflows/extended-platform-compat-canary.yml`は
ARM64 Linux、Intel Mac、Windows ARM64の新規**ネイティブコンパイル専用検証**を開始する。
Linux ARM64のみ追加でNix flakeを評価する。Linux ARM64のHome Manager評価は
`tests/fixtures/machine-linux-arm.nix`を`--override-input machine`で渡し、
Darwinのplaceholderを誤用しない。fixtureのユーザー/ホーム/CPUを評価で照合する。
PRでは本workflowや本ドキュメントの
差分がある場合に実行し、加えて毎週火曜に実行する。fail-fast false相当の独立jobで
結果を明示し、`continue-on-error`は使わない。

このcanaryは当初非requiredとし、各平台でグリーンの証拠と実機テストを得た後に
必要に応じて通常CIのrequired contextへ段階的に移行する。
Windows ARM64ではRust toolchainの起動を実測したが、Windows Native側とWSL側のテストを分離した最新HEADの合格は未確認。
Intel MacでもネイティブCLIテストの合格は新HEADのCIで確認する。
すべてのcanary結果は実行commit SHAとジョブURLを記録し、`abandoned`やjob 0件の失敗を成功と扱わない。

## Nix強化の採用原則

- 単一の`flake.lock`だけでなく、独立template lockを個別に棚卸し
- Nixpkgsの全体更新・バージョン追従は検証用PRで先に評価し、P0バックポートと分離
- `nh`、`nix-index`、`nix-tree`、`nix-melt`、`nix-output-monitor`、
  `devenv`、`omnix`等はすでに既存devShell等にある。重複導入しない
- `sops-nix`やキャッシュサーバー導入は秘密情報管理・コスト・可用性を先に設計
- キャッシュを利用するときはtrusted substituters/公開鍵を最小化し、ハッシュ検証を維持
- 自動更新BotはPR提案まで。release/workflow/installer変更はレビューしてから統合

## 未確定事項
1. Intel Mac用のNix導入provider、既存Nixの安全な取り扱い、nix-darwin出力
2. Windows ARM64でのネイティブRust toolchain/WSL2 helperビルドと互換性
3. Linux ARM64の追加配布アセットとself-update署名/metadata
4. WSL2のsystemdあり/なし、Ubuntuのバージョン別の導入/復旧
5. ディストリビューション別SELinux・store mount・既存dotfiles衝突検査
6. Nix storeのGC・キャッシュ・バックアップ/復旧の権限境界と容量上限
