#!/usr/bin/env bash
# release workflow と PR CI で同一の macOS CLI artifact build を実行する。
# この script を修正した場合は .github/workflows/{check,release}.yml 両方の
# 呼び出し側も同一のまま保つこと (drift が v0.2.0-rc.1/rc.2 の事故原因)。
#
# host toolchain (cargo) で build する。nix build 産 binary は /nix/store の
# libiconv に link し Nix-less Mac で起動できない (rc.3 で gate が実際に検出)。
# Linux musl build と同じ理由で host build とする。
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cargo build --release -p schneeforge

BIN="target/release/schneeforge"
"$BIN" --version
"$BIN" doctor >/dev/null

"$SCRIPT_DIR/check-macos-portability.sh" "$BIN"

echo "OK: macOS release CLI built and verified"
