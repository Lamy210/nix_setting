#!/usr/bin/env bats

# install.sh の Managed Nix 統合 (issue #14 作業項目 3) の unit test。
# network access は不要 (curl / sha256sum は stub して関数 logic のみ検証)。
#
# shellcheck disable=SC2030,SC2031  # bats の @test は subshell で動くため export の影響は test 内で完結する
# shellcheck disable=SC2329  # uname() は eval した install.sh 関数から呼ばれる

INSTALL_SH="$BATS_TEST_DIRNAME/../install.sh"

# install.sh から resolver / fetch 関数だけを取り出す
# (main flow の `echo "=== nix_setting installer ==="` 以降は読み込まない)
INSTALL_FUNCTIONS="$(sed -n '1,/^# --- end inline resolver ---$/p' "$INSTALL_SH")"

# stub 環境を setup して関数を eval する。
#   $1: CHECKSUMS に置く sha256 (空なら entry 無し)
#   $2: sha256sum stub が返す値 (default = $1)
load_stubbed() {
  local checksums_sha="${1:-}"
  local actual_sha="${2:-$checksums_sha}"
  mkdir -p "$BATS_TEST_TMPDIR/bin"

  # curl stub: URL を log に記録。-o で指定された file へ body を書く
  cat >"$BATS_TEST_TMPDIR/bin/curl" <<EOF
#!/usr/bin/env bash
echo "\$*" >>"$BATS_TEST_TMPDIR/curl.log"
out=""
prev=""
for a in "\$@"; do
  if [ "\$prev" = "-o" ]; then out="\$a"; fi
  prev="\$a"
done
case "\$*" in
  *CHECKSUMS.txt)
    if [ -n "$checksums_sha" ]; then
      printf '%s  dist/schneeforge-x86_64-linux/schneeforge-x86_64-linux\\n' "$checksums_sha" >"\$out"
    fi
    ;;
  *schneeforge-*)
    printf 'fake-binary\\n' >"\$out"
    ;;
esac
exit 0
EOF

  # sha256sum stub
  cat >"$BATS_TEST_TMPDIR/bin/sha256sum" <<EOF
#!/usr/bin/env bash
echo "$actual_sha  \$1"
EOF

  chmod +x "$BATS_TEST_TMPDIR/bin/curl" "$BATS_TEST_TMPDIR/bin/sha256sum"
  PATH="$BATS_TEST_TMPDIR/bin:$PATH"
  eval "$INSTALL_FUNCTIONS"
}

@test "install.sh has no direct nixos.org installer invocation" {
  # Managed Nix 統合後、curl|sh で nixos.org installer を直接実行していないこと
  run grep -n "nixos.org/nix/install" "$INSTALL_SH"
  [ "$status" -ne 0 ]
}

@test "install.sh calls schneeforge nix install via sudo" {
  run grep -n 'sudo env NIX_SETTING_DIR.*nix install' "$INSTALL_SH"
  [ "$status" -eq 0 ]
}

@test "install.sh passes /dev/tty to sudo when stdin is piped" {
  # curl|bash で stdin が pipe の場合、CLI の D8 確認 (TTY 必須) が機能するよう
  # /dev/tty を繋いでいること
  run grep -n 'nix install </dev/tty' "$INSTALL_SH"
  [ "$status" -eq 0 ]
}

@test "fetch_schneeforge_binary downloads expected asset (linux x86_64)" {
  uname() {
    case "$1" in
      -s) echo "Linux" ;;
      -m) echo "x86_64" ;;
    esac
  }
  export SCHNEEFORGE_VERSION="v9.9.9"
  load_stubbed "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  bin="$(fetch_schneeforge_binary)"
  # 戻り値の path に linux asset 名が含まれること
  case "$bin" in
    *schneeforge-x86_64-linux) ;;
    *) return 1 ;;
  esac
  # linux asset を download していること
  grep -q "schneeforge-x86_64-linux" "$BATS_TEST_TMPDIR/curl.log"
}

@test "fetch_schneeforge_binary rejects unsupported os" {
  uname() {
    case "$1" in
      -s) echo "FreeBSD" ;;
      -m) echo "x86_64" ;;
    esac
  }
  load_stubbed "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  run fetch_schneeforge_binary
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "unsupported OS"
}

@test "fetch_schneeforge_binary rejects unsupported arch" {
  uname() {
    case "$1" in
      -s) echo "Linux" ;;
      -m) echo "riscv64" ;;
    esac
  }
  load_stubbed "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  run fetch_schneeforge_binary
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "unsupported arch"
}

@test "fetch_schneeforge_binary fails when checksum entry missing" {
  uname() {
    case "$1" in
      -s) echo "Linux" ;;
      -m) echo "x86_64" ;;
    esac
  }
  # CHECKSUMS に entry を書かない (checksums_sha 空)
  export SCHNEEFORGE_VERSION="v9.9.9"
  load_stubbed ""
  run fetch_schneeforge_binary
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "entry が無い"
}

@test "fetch_schneeforge_binary detects sha256 mismatch" {
  uname() {
    case "$1" in
      -s) echo "Linux" ;;
      -m) echo "x86_64" ;;
    esac
  }
  # CHECKSUMS 値と sha256sum 実測値を変える
  export SCHNEEFORGE_VERSION="v9.9.9"
  load_stubbed \
    "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd" \
    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
  run fetch_schneeforge_binary
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "sha256 mismatch"
}
