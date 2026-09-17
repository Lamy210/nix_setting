# Windows / WSL2 experimental support

SchneeForge の Windows 対応は、**Windows を Nix の実行プラットフォームとして扱う機能ではありません**。
Windows 上では native CLI を launcher/control-plane として使い、Nix・repository・state・tool discovery を必要とする操作は、選択した WSL2 Linux distribution 内の `schneeforge` helper へ委譲します。

## 現在のサポート範囲

- Windows native CLI launcher
- WSL2 distribution の検出・選択
- launcher と Linux helper の protocol / application version / OS / architecture handshake
- `wsl.exe -d <distro> --exec schneeforge <args...>` 形式のshell-free argv委譲
- Windows host 向け `doctor`
- `--help` / `--version` の local 実行
- Windows 上の `self-update` の明示的な unsupported エラー

Nix 側の `Platform` / `ConfigurationTarget` / `system` は従来どおり macOS / Linux のみです。

## 前提条件

1. Windows に WSL が導入されていること。
2. 使用する distribution が登録済みで **WSL2** であること。WSL1 は fail-closed で拒否します。
3. 選択した WSL2 distribution 内で Linux 版 `schneeforge` が `PATH` から実行できること。
4. Windows launcher と WSL helper の SchneeForge application version が一致すること。
5. helper が bridge protocol version と Linux identity を正しく返せること。

初期実装では WSL、distribution、Linux helper の自動インストール・自動更新は行いません。

## Distribution の選択

選択優先順位は固定です。

1. CLI `--wsl-distro <DISTRO>`
2. `SCHNEEFORGE_WSL_DISTRO`
3. WSL が報告する default distribution

例:

```powershell
schneeforge --wsl-distro Ubuntu apply

$env:SCHNEEFORGE_WSL_DISTRO = "Debian"
schneeforge status
```

明示した distribution が未登録、WSL1、version不明、defaultが存在しない、といった状態では別distributionへ暗黙fallbackせずエラーにします。

## Repository path

`--repo` を省略した場合、repository解決はWindows側では行わず、選択したWSL2内のLinux helperが既存のLinuxルールで解決します。

明示する場合は **WSL内のabsolute Linux path** のみを受け付けます。

```powershell
schneeforge --repo /home/alice/nix_setting status
```

次のようなWindows-native pathは初期実装では拒否します。

```text
C:\Users\alice\nix_setting
\\server\share\nix_setting
```

`wslpath` による暗黙変換や `/mnt/c` への自動変換は行いません。repositoryはWSL filesystem側に置く運用を推奨します。

## Command behavior

| Command / option | Windows host behavior |
| --- | --- |
| `--help`, `--version` | Windows launcherでlocal実行。WSL不要 |
| `doctor` | Windows/WSL inventory、選択distribution、helper compatibilityをhost側から診断 |
| `self-update` | 初期実装ではunsupported。Linux helperだけを更新する動作はしない |
| `status`, `plan`, `apply`, `rollback` など | WSL2 Linux helperへ委譲 |

委譲時は shell command string を生成せず、`wsl.exe` の `--exec` で `schneeforge` と各引数を直接渡します。空白やshell metacharacterを含む引数も1つのargv entryとして保持します。

## Doctor

```powershell
schneeforge doctor
schneeforge --wsl-distro Ubuntu doctor
```

`doctor` は helper が未導入・version不一致でもWindows/WSL側の診断を残します。WSL availability、registered distribution、WSL version、selection source、helper compatibility、backend readinessを確認できます。

## 現時点のnon-goals

- Windows Desktop / Tauri GUI
- native Windows Nix backend
- Windows package manager backend（winget / Scoop等）
- WSL / distribution の自動インストール
- WSL helper の自動インストール・自動更新
- WSL1 fallback
- Windows path ↔ Linux path の暗黙変換
- Windows release asset / installer
- Windows `self-update`

## CI

PR CIには `windows-2025` の `windows-check` を置き、次を検証します。

- workspace compile
- core test target のWindows compile
- Windows backend contract tests
- launcher unit tests
- host-aware CLI contract tests
- native `.exe` の `--version` / `--help` smoke

`windows-check` は現時点では **non-required** で、既存 `ci-required` や release artifact gate の依存には追加していません。

加えて `.github/workflows/windows-wsl-canary.yml` を独立した non-required canary として用意しています。これは PR では実行せず、`develop` push、週次 schedule、manual dispatch で `windows-2025` 上に Ubuntu WSL2 を起動し、transport-only helper fixture を使って次を実WSL境界で確認します。

- `--wsl-distro Ubuntu` による明示selector
- `SCHNEEFORGE_WSL_DISTRO` によるenvironment selector
- 空白・shell metacharacterを含むargvの保持
- Linux helper handshake
- delegated exit status `23` の保持

canary は `check.yml` の `ci-required`、release artifact gate、`release.yml` の依存には含めません。Windows release配布を開始する前に、別途distribution/update設計とrelease gateを追加します。
