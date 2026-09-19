# Design: macOS Managed Nix lifecycle acceptance helper

## D1. Manual-only / non-required

workflow は `workflow_dispatch` のみで起動する。
PR、push、schedule では自動実行せず、`check.yml` / `release.yml` /
`ci-required` / `macos-check` の dependency にしない。

理由:
- Managed Nix install/uninstall は `/nix` と system service を変更する
- runner time / network cost が大きい
- Final Acceptance は release candidate 単位の明示実行が適切

## D2. Hosted Apple Silicon fresh-host contract

runner は `macos-15` を使用し、script 冒頭で以下を fail-closed 検証する:

- `GITHUB_ACTIONS=true`
- `uname -m == arm64`
- `/nix` が存在しない
- `nix` command が解決できない

既に Nix がある環境では「既存環境を destructive test に使う」ことを避けるため
即 failure とする。

## D3. Release artifact を authority にする

workflow checkout は contract/script を取得するためだけに使う。
検証対象は input `tag` の release asset:

- `schneeforge-aarch64-darwin`
- `CHECKSUMS.txt`

binary は CHECKSUMS の SHA256 と一致した場合のみ root 実行する。
CLI command は repository checkout 外の temporary working directory から実行し、
`NIX_SETTING_DIR` を unset して release binary の embedded manifest を使う。
これにより develop checkout の `bootstrap-manifest.toml` が混入しない。

## D4. TTY を要求しない CLI lifecycle に限定

CI では `sudo <release-binary> nix install --yes` を使用する。
`--yes` は automation 用として既存 spec が許可しており、upstream
`--no-confirm` に対する SchneeForge の確認責任を CI では明示的に skip する。

したがって本 helper は `install.sh` の `/dev/tty` D8 prompt を
Final Acceptance から置き換えない。manual checklist の gate B3 は残す。

## D5. Lifecycle coverage

1. release binary + checksum download / verify
2. Managed Nix install
3. receipt / ownership record
4. `nix store ping` / flakes
5. `schneeforge nix doctor`
6. second install rejection (`ExistingNixDetected`)
7. uninstall / semantic cleanup (Nix mount / receipt / store / build users / nix-daemon が消えること。bare synthetic `/nix` path は許容)
8. reinstall
9. final uninstall / 同一 semantic cleanup

nix-darwin apply は実行しないため、nix-darwin uninstaller の検証は本 helper の
scope 外。manual full bootstrap acceptance で扱う。

## D6. Failure evidence

各主要 command の stdout/stderr を temporary acceptance directory に log し、
workflow は `if: always()` でその directory を artifact upload する。
ログに secret を含める設計にはしないが、将来環境情報を追加する場合も
username / hostname / private path を public artifact へ出さない。

## D7. ADR status boundary

workflow success は CLI lifecycle の automated evidence であり、
ADR-0001 Final Acceptance の十分条件ではない。
Finder GUI gates A2/E/I-3 と install.sh interactive path を含む manual checklist
完了後にのみ ADR を `Accepted` へ昇格する。

## D8. Post-merge execution

GitHub の manual workflow dispatch は default branch 上の workflow を基準にする
ため、実装 PR merge 後に input tag `v0.2.0-rc.7` で1回実行し、成功 run を
archive PR で記録する。
