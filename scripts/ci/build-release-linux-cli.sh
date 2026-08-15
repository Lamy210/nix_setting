#!/usr/bin/env bash
# release workflow と PR CI で同一の Linux musl static CLI build を実行する。
# 要件: musl-tools (musl-gcc) と rust toolchain (x86_64-unknown-linux-musl target)。
# v0.2.0-rc.1 で「nix build 産 binary が Nix-less machine で動かない」問題の対策。
set -euo pipefail

cargo build --release -p schneeforge --target x86_64-unknown-linux-musl

BIN="target/x86_64-unknown-linux-musl/release/schneeforge"
"$BIN" --version
"$BIN" doctor >/dev/null

# static ELF であること (INTERP が無い = どんな glibc 環境でも動く)
if readelf -l "$BIN" | grep -q INTERP; then
  echo "ERROR: release CLI is not fully static:" >&2
  readelf -l "$BIN" | grep INTERP >&2
  exit 1
fi

echo "OK: Linux musl release CLI built and verified"
