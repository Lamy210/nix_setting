#!/usr/bin/env bash
# release workflow と PR CI で同一の macOS CLI artifact build を実行する。
# この script を修正した場合は .github/workflows/{check,release}.yml 両方の
# 呼び出し側も同一のまま保つこと (drift が v0.2.0-rc.1/rc.2 の事故原因)。
#
# host toolchain (cargo) で build する。nix build 産 binary は /nix/store の
# libiconv に link し Nix-less Mac で起動できない (rc.3 で gate が実際に検出)。
# Linux musl build と同じ理由で host build とする。
set -euo pipefail

cargo build --release -p schneeforge

BIN="target/release/schneeforge"
"$BIN" --version
"$BIN" doctor >/dev/null

# direct dylib 依存に /nix/store が無いこと (otool -L の依存行は indent される)
if otool -L "$BIN" | grep -qE '^[[:space:]]*/nix/store/'; then
  echo "ERROR: release CLI links against /nix/store (not portable):" >&2
  otool -L "$BIN" >&2
  exit 1
fi

# LC_RPATH 経由の /nix/store も弾く (@rpath 依存 + rpath 解決で実行時破綻するため)
if otool -l "$BIN" | awk '
  $1 == "cmd" && $2 == "LC_RPATH" { in_rpath = 1; next }
  in_rpath && $1 == "path" { print $2; in_rpath = 0 }
' | grep -q '^/nix/store/'; then
  echo "ERROR: release CLI contains /nix/store LC_RPATH (not portable):" >&2
  otool -l "$BIN" >&2
  exit 1
fi

echo "OK: macOS release CLI built and verified"
