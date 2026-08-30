#!/usr/bin/env bats

# scripts/ci/build-release-*.sh の gate logic を検証する。
# 実 binary build は CI job が担うため、ここでは grep pattern の
# 検出力 (false-negative / false-positive) を fixture で保証する。

# otool -L gate と同じ pattern。otool の dependency 行は行頭が
# indent されるため、^ 固定だと /nix/store 依存を見逃す (RC.2 follow-up)
NIX_STORE_PATTERN='^[[:space:]]*/nix/store/'

# build-release-macos-cli.sh の LC_RPATH 抽出 awk と同一の logic
extract_rpaths() {
  awk '
    $1 == "cmd" && $2 == "LC_RPATH" { in_rpath = 1; next }
    in_rpath && $1 == "path" { print $2; in_rpath = 0 }
  '
}

@test "otool gate pattern rejects indented /nix/store dependency" {
  output="$(printf 'result/bin/schneeforge:\n\t/nix/store/xxxx-libfoo.dylib (compatibility version)\n' \
    | grep -E "$NIX_STORE_PATTERN")"
  [ -n "$output" ]
}

@test "otool gate pattern rejects non-indented /nix/store dependency" {
  output="$(printf '/nix/store/xxxx-libfoo.dylib\n' | grep -E "$NIX_STORE_PATTERN")"
  [ -n "$output" ]
}

@test "otool gate pattern allows system libSystem dependency" {
  run sh -c "printf 'result/bin/schneeforge:\n\t/usr/lib/libSystem.B.dylib\n' | grep -E '$NIX_STORE_PATTERN'"
  [ "$status" -ne 0 ]
}

# LC_RPATH gate: @rpath 依存 + LC_RPATH /nix/store の組合せは
# otool -L には /nix/store が現れないため -l での抽出が必須
@test "LC_RPATH extraction rejects /nix/store rpath" {
  output="$(printf 'Load command 12\n      cmd LC_RPATH\n      cmdsize 32\n      path /nix/store/xxxx-libfoo/lib (offset 12)\n' \
    | extract_rpaths | grep '^/nix/store/')"
  [ -n "$output" ]
}

@test "LC_RPATH extraction allows /usr/local/lib rpath" {
  run sh -c "printf 'Load command 12\n      cmd LC_RPATH\n      cmdsize 32\n      path /usr/local/lib (offset 12)\n' \
    | extract_rpaths | grep '^/nix/store/'"
  [ "$status" -ne 0 ]
}

# readelf INTERP gate と同等の検査 (Linux static binary)
@test "INTERP gate pattern detects dynamic interpreter segment" {
  output="$(printf '  INTERP    0x0000000000000318\n' | grep -q INTERP && echo matched)"
  [ "$output" = "matched" ]
}

@test "INTERP gate pattern passes when no INTERP segment" {
  run sh -c "printf '  LOAD    0x0000000000000000\n' | grep -q INTERP"
  [ "$status" -ne 0 ]
}

# --- check-macos-portability.sh の arm64 判定 (RC.4 DMG 事故で追加) ---

@test "arm64 gate accepts arm64 architecture string" {
  output="$(printf 'arm64\n' | grep -qE '^(arm64|\*arm64)' && echo matched)"
  [ "$output" = "matched" ]
}

@test "arm64 gate rejects x86_64 architecture string" {
  run sh -c "printf 'x86_64\n' | grep -qE '^(arm64|\*arm64)'"
  [ "$status" -ne 0 ]
}

# --- build-release-macos-dmg.sh の pin 検証 ---

@test "tauri CLI sha256 pin is exact length" {
  SHA="$(grep '^TAURI_CLI_SHA256=' scripts/ci/build-release-macos-dmg.sh | cut -d'"' -f2)"
  [ "${#SHA}" -eq 64 ]
}

@test "tauri CLI download URL embeds pinned version" {
  VERSION="$(grep '^TAURI_CLI_VERSION=' scripts/ci/build-release-macos-dmg.sh | cut -d'"' -f2)"
  URL="$(grep '^TAURI_CLI_URL=' scripts/ci/build-release-macos-dmg.sh | sed "s/\${TAURI_CLI_VERSION}/$VERSION/" | cut -d'"' -f2)"
  echo "$URL" | grep -q "tauri-cli-v${VERSION}/"
}

@test "dmg script gates mounted app binary not raw build output" {
  grep -q 'hdiutil attach' scripts/ci/build-release-macos-dmg.sh
  grep -q 'check-macos-portability.sh' scripts/ci/build-release-macos-dmg.sh
  grep -q 'CFBundleShortVersionString' scripts/ci/build-release-macos-dmg.sh
}

# --- DMG 内 CLI sidecar (GUI escalation 先) の gate ---

@test "dmg script builds CLI before tauri bundle (externalBin source)" {
  # build script は target/<profile>/schneeforge を stage 元にするため、
  # tauri build の前に CLI build が必要
  CLI_BUILD_LINE="$(grep -n 'cargo build --release -p schneeforge' scripts/ci/build-release-macos-dmg.sh | cut -d: -f1)"
  TAURI_BUILD_LINE="$(grep -n 'TAURI_BIN. build' scripts/ci/build-release-macos-dmg.sh | cut -d: -f1)"
  [ -n "$CLI_BUILD_LINE" ]
  [ -n "$TAURI_BUILD_LINE" ]
  [ "$CLI_BUILD_LINE" -lt "$TAURI_BUILD_LINE" ]
}

@test "dmg script verifies CLI sidecar inside mounted app" {
  # tauri 2.x は bundle 時に triple suffix を除去する
  grep -q 'MacOS/schneeforge-cli' scripts/ci/build-release-macos-dmg.sh
}

# scripts/ci/slsa_predicate.py の出力構造検証 (add-release-attestation-bundles)。
# cosign 署名は OIDC 依存で PR CI では実行できないため、純粋 logic である
# predicate 生成のみ dummy 値で検証する (D6)
DUMMY_SHA="0123456789abcdef0123456789abcdef01234567"

@test "slsa predicate contains builder id, materials sha1 and entrypoint" {
  tmp="$(mktemp -d)"
  python3 scripts/ci/slsa_predicate.py v9.9.9-rc.9 "$DUMMY_SHA" "refs/tags/v9.9.9-rc.9" "$tmp/predicate.json"
  python3 - "$tmp/predicate.json" "$DUMMY_SHA" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    p = json.load(f)
assert p["builder"]["id"] == "https://github.com/Lamy210/nix_setting/.github/workflows/release.yml@refs/tags/v9.9.9-rc.9", p["builder"]
assert p["materials"][0]["digest"]["sha1"] == sys.argv[2], p["materials"]
assert p["invocation"]["configSource"]["entryPoint"] == ".github/workflows/release.yml"
assert p["invocation"]["configSource"]["digest"]["sha1"] == sys.argv[2]
PY
}

@test "slsa predicate rejects malformed source sha" {
  tmp="$(mktemp -d)"
  run python3 scripts/ci/slsa_predicate.py v9.9.9 "not-a-sha" "refs/tags/v9.9.9" "$tmp/predicate.json"
  [ "$status" -ne 0 ]
}
