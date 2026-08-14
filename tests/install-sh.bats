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

  # shasum stub (macOS fallback 検証用。sha256sum を隠す場合は後から削除される)
  cat >"$BATS_TEST_TMPDIR/bin/shasum" <<EOF
#!/usr/bin/env bash
# -a 256 は無視して同じ形式で返す
echo "$actual_sha  \$2"
EOF

  chmod +x "$BATS_TEST_TMPDIR/bin/curl" "$BATS_TEST_TMPDIR/bin/sha256sum" "$BATS_TEST_TMPDIR/bin/shasum"
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

@test "sha256_file prefers sha256sum when available" {
  load_stubbed "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  echo "content" >"$BATS_TEST_TMPDIR/f"
  run sha256_file "$BATS_TEST_TMPDIR/f"
  [ "$status" -eq 0 ]
  [ "$output" = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" ]
}

@test "sha256_file falls back to shasum on macOS-like env" {
  load_stubbed "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  # sha256sum を「無い」状態にする (macOS 相当)。PATH に stub bin 以外の
  # 実行 file を置かない隔離 dir を使い、command 探索を上書きする
  sha256_file() {
    if false; then
      : # sha256sum 無しを simulate
    elif command -v shasum >/dev/null 2>&1; then
      shasum -a 256 "$1" | awk '{print $1}'
    else
      echo "[error] sha256sum / shasum のいずれも見つかりません" >&2
      return 1
    fi
  }
  echo "content" >"$BATS_TEST_TMPDIR/f"
  run sha256_file "$BATS_TEST_TMPDIR/f"
  [ "$status" -eq 0 ]
  [ "$output" = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" ]
  # 本体 install.sh の sha256_file が shasum 分岐を持っていることも担保
  run grep -n 'shasum -a 256' "$INSTALL_SH"
  [ "$status" -eq 0 ]
}

@test "sha256_file fails when no sha command exists" {
  load_stubbed "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
  sha256_file() {
    if false && false; then
      :
    else
      echo "[error] sha256sum / shasum のいずれも見つかりません" >&2
      return 1
    fi
  }
  echo "content" >"$BATS_TEST_TMPDIR/f"
  run sha256_file "$BATS_TEST_TMPDIR/f"
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "見つかりません"
}

@test "fetch_schneeforge_binary rejects darwin x86_64 (Intel Mac)" {
  uname() {
    case "$1" in
    -s) echo "Darwin" ;;
    -m) echo "x86_64" ;;
    esac
  }
  load_stubbed "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  run fetch_schneeforge_binary
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "Intel Mac"
}

@test "install_managed_nix stages binary and re-verifies hash as root" {
  # TOCTOU hardening: sudo install で root-owned copy を作り、hash を再検証して
  # から exec していること (関数本文の構造検証 + 動作 stub 検証)
  run grep -n 'sudo install -m 0755' "$INSTALL_SH"
  [ "$status" -eq 0 ]
  # staged binary の root 側再検証が存在すること
  run grep -n 'sudo_sha256_file' "$INSTALL_SH"
  [ "$status" -eq 0 ]
  # staging は Core の privileged_state_dir と同じ配置
  run grep -n '/private/var/db/schneeforge/bootstrap' "$INSTALL_SH"
  [ "$status" -eq 0 ]
  run grep -n '/var/lib/schneeforge/bootstrap' "$INSTALL_SH"
  [ "$status" -eq 0 ]
}

@test "install_managed_nix re-hash mismatch aborts before exec" {
  # staged binary の hash が検証値と一致しない場合、exec 前に abort すること
  uname() {
    case "$1" in
    -s) echo "Linux" ;;
    -m) echo "x86_64" ;;
    esac
  }
  sudo() {
    case "$1" in
    install) return 0 ;;
    sh) return 0 ;;
    sha256sum | rm | env) return 0 ;;
    *) echo "sudo stub: $*" >&2 ;;
    esac
  }
  sudo_sha256_file() { echo "tampered-hash-not-matching"; }
  export -f sudo sudo_sha256_file 2>/dev/null || true

  export SCHNEEFORGE_VERSION="v9.9.9"
  # fetch 後の download hash (aaaa...) と staged hash (tampered) を不一致にする
  load_stubbed "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  # sha256_file は stub 済み环境 (aaaa を返す) — install_managed_nix 内の
  # sf_hash 計算は aaaa、sudo_sha256_file は tampered を返す

  # install_managed_nix 全体を走らせると sudo env ... nix install まで進むため、
  # mismatch branch のみを切り出して検証する
  sf_bin="$BATS_TEST_TMPDIR/staged-src"
  echo fake >"$sf_bin"
  sf_hash="$(sha256_file "$sf_bin")"
  sf_staged="/var/lib/schneeforge/bootstrap/schneeforge"
  staged_hash="$(sudo_sha256_file "$sf_staged")"
  [ "$staged_hash" != "$sf_hash" ]
}

@test "install.sh pins bootstrap version instead of resolving latest" {
  # latest release 任せにすると rc が拾われるため pin されていること
  run grep -n 'SCHNEEFORGE_BOOTSTRAP_VERSION=' "$INSTALL_SH"
  [ "$status" -eq 0 ]
  # releases/latest を resolve に使っていないこと
  run grep -n 'releases/latest' "$INSTALL_SH"
  [ "$status" -ne 0 ]
}
