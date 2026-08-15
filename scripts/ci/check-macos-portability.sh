#!/usr/bin/env bash
# macOS release binary の portability gate (CLI と DMG 内 desktop binary の共通検査)。
#
# 検査対象は build 済み Mach-O 1 個。以下を全て満たすことを保証する:
#   1. arm64 (aarch64) であること
#   2. direct dylib 依存 (otool -L) に /nix/store が無いこと
#      (nix build 産 binary は libiconv 等が /nix/store に link し、
#       Nix 未導入 Mac で dyld 解決失敗により起動できない。rc.2 で実際に発生し、
#       RC.4 では CLI 修正後も DMG 経路に残っていた)
#   3. LC_RPATH (otool -l) に /nix/store が無いこと
#      (@rpath 依存 + /nix/store rpath は otool -L に現れないため -l での抽出が必須)
#
# この script を修正した場合は呼び出し側 (build-release-macos-{cli,dmg}.sh) の
# 両方が同一のまま保つこと。gate pattern の検証は tests/ci-scripts.bats にある。
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 <macho-binary>" >&2
  exit 2
fi

BIN="$1"

if [ ! -f "$BIN" ]; then
  echo "ERROR: binary not found: $BIN" >&2
  exit 2
fi

# 1. architecture
ARCH="$(lipo -archs "$BIN" 2>/dev/null || true)"
if [ -z "$ARCH" ]; then
  # lipo が無い環境では file 出力の fallback
  ARCH="$(file "$BIN")"
  case "$ARCH" in
  *arm64*) ARCH="arm64" ;;
  *) ARCH="unknown" ;;
  esac
fi
case "$ARCH" in
arm64* | *arm64*) : ;;
*)
  echo "ERROR: release binary is not arm64: $ARCH" >&2
  exit 1
  ;;
esac

# 2. direct dylib 依存に /nix/store が無いこと (otool -L の依存行は indent される)
if otool -L "$BIN" | grep -qE '^[[:space:]]*/nix/store/'; then
  echo "ERROR: binary links against /nix/store (not portable):" >&2
  otool -L "$BIN" >&2
  exit 1
fi

# 3. LC_RPATH 経由の /nix/store も弾く
if otool -l "$BIN" | awk '
  $1 == "cmd" && $2 == "LC_RPATH" { in_rpath = 1; next }
  in_rpath && $1 == "path" { print $2; in_rpath = 0 }
' | grep -q '^/nix/store/'; then
  echo "ERROR: binary contains /nix/store LC_RPATH (not portable):" >&2
  otool -l "$BIN" >&2
  exit 1
fi

echo "OK: $BIN is portable (arm64, no /nix/store deps)"
