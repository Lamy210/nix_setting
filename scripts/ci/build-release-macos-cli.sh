#!/usr/bin/env bash
# release workflow と PR CI で同一の macOS CLI artifact build を実行する。
# この script を修正した場合は .github/workflows/{check,release}.yml 両方の
# 呼び出し側も同一のまま保つこと (drift が v0.2.0-rc.2 の事故原因)。
set -euo pipefail

# build (buildRustPackage の checkPhase で cargo test も走る = test も gate)
nix build .#schneeforge

# smoke
./result/bin/schneeforge --version
./result/bin/schneeforge doctor >/dev/null

# portability: /nix/store 依存の dylib を持たないこと (Nix-less machine で動く)。
# otool -L の dependency 行は行頭が indent されるため ^/nix/store では検出できない
if otool -L result/bin/schneeforge | grep -qE '^[[:space:]]*/nix/store/'; then
  echo "ERROR: release CLI links against /nix/store (not portable):" >&2
  otool -L result/bin/schneeforge >&2
  exit 1
fi

echo "OK: macOS release CLI built and verified"
