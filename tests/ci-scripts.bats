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
