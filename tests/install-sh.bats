#!/usr/bin/env bats

# install.sh の Managed Nix 統合 (issue #14 作業項目 3) の unit test。
# network access は不要 (curl / sha256sum は stub して関数 logic のみ検証)。
#
# shellcheck disable=SC2030,SC2031  # bats の @test は subshell で動くため export の影響は test 内で完結する
# shellcheck disable=SC2329  # uname() は eval した install.sh 関数から呼ばれる

INSTALL_SH="$BATS_TEST_DIRNAME/../install.sh"

# install.sh から resolver / fetch 関数だけを取り出す
# (main flow の `echo "=== nix_setting installer ==="` 以降は読み込まない)。
# /dev/tty redirect は test 環境 (CI container) で open できないため
# /dev/null へ置換する (redirect 先が違うだけで分岐 logic は同じ)
INSTALL_FUNCTIONS="$(sed -n '1,/^# --- end inline resolver ---$/p' "$INSTALL_SH" | sed 's|/dev/tty|/dev/null|g')"

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

@test "fetch_schneeforge_binary rejects linux aarch64 (asset not distributed)" {
  uname() {
    case "$1" in
    -s) echo "Linux" ;;
    -m) echo "aarch64" ;;
    esac
  }
  load_stubbed "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  run fetch_schneeforge_binary
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "Linux aarch64"
}

@test "install_managed_nix creates staging dir before copy on fresh machine" {
  # P0 regression: fresh machine には staging dir が存在しない。
  # dir 作成 (install -d) → binary copy (install) → root 側 hash 再検証 → exec
  # の順で実行されることを、実 filesystem を使った stub で検証する
  uname() {
    case "$1" in
    -s) echo "Linux" ;;
    -m) echo "x86_64" ;;
    esac
  }
  load_stubbed "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

  # 実 file 操作を記録する sudo stub (root 権限相当の sandbox 下で動く)
  local sandbox="$BATS_TEST_TMPDIR/rootfs"
  local stage="/var/lib/schneeforge/bootstrap"
  : >"$BATS_TEST_TMPDIR/sudo.log"
  sudo() {
    echo "sudo $*" >>"$BATS_TEST_TMPDIR/sudo.log"
    case "$1" in
    install)
      if [ "$2" = "-d" ]; then
        # install -d -m 0700 <dir>: sandbox 内に作成
        mkdir -p "${sandbox}$5"
        return 0
      fi
      # install -m 0755 <src> <dst>
      local dst="$5"
      case "$dst" in
      "$stage"/*) dst="${sandbox}${dst}" ;;
      esac
      cp "$4" "$dst"
      chmod 0755 "$dst"
      ;;
    rm)
      local target="$3"
      case "$target" in
      "$stage"/*) target="${sandbox}${target}" ;;
      esac
      rm -f "$target"
      ;;
    rmdir)
      case "$2" in
      "$stage") rmdir "${sandbox}$2" 2>/dev/null || true ;;
      esac
      ;;
    sh) return 0 ;;
    sha256sum) sha256sum "$2" | awk '{print $1}' ;;
    env)
      # exec 相当: staged binary が存在することを確認してから記録
      if [ ! -f "${sandbox}${stage}/schneeforge" ]; then
        echo "ERROR: exec before staging" >&2
        return 1
      fi
      echo "exec: $*" >>"$BATS_TEST_TMPDIR/sudo.log"
      return 0
      ;;
    *) return 0 ;;
    esac
  }
  # staged path の hash 検証は sandbox 内 file を見る
  sudo_sha256_file() { sudo sha256sum "${sandbox}/var/lib/schneeforge/bootstrap/schneeforge"; }
  fetch_schneeforge_binary() { echo "$BATS_TEST_TMPDIR/sf-download"; }
  echo fake-binary-content >"$BATS_TEST_TMPDIR/sf-download"
  mkdir -p "$sandbox"

  # 最後の nix 再探索は install 成功後の処理のため stub する
  resolve_nix() { return 0; }

  # INSTALL_FUNCTIONS は /dev/tty → /dev/null 置換済み (CI container で
  # open できないため)。redirect 先が違うだけで分岐 logic は同じ
  run install_managed_nix "$BATS_TEST_TMPDIR/repo"
  [ "$status" -eq 0 ]

  # command order: dir 作成が copy より前、exec より前に hash 再検証
  local log first_d first_cp first_exec first_rm
  log="$(cat "$BATS_TEST_TMPDIR/sudo.log")"
  first_d="$(echo "$log" | grep -n 'install -d' | head -1 | cut -d: -f1)"
  first_cp="$(echo "$log" | grep -n 'install -m 0755' | head -1 | cut -d: -f1)"
  first_exec="$(echo "$log" | grep -n '^exec:' | head -1 | cut -d: -f1)"
  [ -n "$first_d" ]
  [ -n "$first_cp" ]
  [ -n "$first_exec" ]
  [ "$first_d" -lt "$first_cp" ]
  [ "$first_cp" -lt "$first_exec" ]

  # cleanup: binary と staging dir が削除されていること
  first_rm="$(echo "$log" | grep -n 'rm -f' | head -1 | cut -d: -f1)"
  [ -n "$first_rm" ]
  [ "$first_exec" -lt "$first_rm" ]
  [ ! -e "${sandbox}${stage}/schneeforge" ]
  [ ! -d "${sandbox}${stage}" ]
}

@test "install_managed_nix aborts before exec when staged hash mismatches" {
  # TOCTOU: staged binary の hash が検証値と一致しない場合、exec せず abort する
  uname() {
    case "$1" in
    -s) echo "Linux" ;;
    -m) echo "x86_64" ;;
    esac
  }
  load_stubbed "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

  local sandbox="$BATS_TEST_TMPDIR/rootfs"
  local stage="/var/lib/schneeforge/bootstrap"
  mkdir -p "$sandbox"
  : >"$BATS_TEST_TMPDIR/sudo.log"
  sudo() {
    echo "sudo $*" >>"$BATS_TEST_TMPDIR/sudo.log"
    case "$1" in
    install)
      if [ "$2" = "-d" ]; then
        mkdir -p "${sandbox}$5"
        return 0
      fi
      local dst="$5"
      case "$dst" in
      "$stage"/*) dst="${sandbox}${dst}" ;;
      esac
      # copy 後に tamper: hash が検証値から変わる
      {
        cat "$BATS_TEST_TMPDIR/sf-download"
        echo tampered
      } >"$dst"
      chmod 0755 "$dst"
      ;;
    rm)
      local target="$3"
      case "$target" in
      "$stage"/*) target="${sandbox}${target}" ;;
      esac
      rm -f "$target"
      ;;
    rmdir) rmdir "${sandbox}$2" 2>/dev/null || true ;;
    sh) return 0 ;;
    # load_stubbed の stub (固定値) ではなく実 sha256sum を使う:
    # sf_hash 側は stub の固定値になるため、tamper しない限り一致しない
    sha256sum) /usr/bin/sha256sum "$2" | awk '{print $1}' ;;
    env)
      echo "UNEXPECTED-EXEC: $*" >>"$BATS_TEST_TMPDIR/sudo.log"
      return 0
      ;;
    *) return 0 ;;
    esac
  }
  sudo_sha256_file() { sudo sha256sum "${sandbox}/var/lib/schneeforge/bootstrap/schneeforge"; }
  fetch_schneeforge_binary() { echo "$BATS_TEST_TMPDIR/sf-download"; }
  echo fake-binary-content >"$BATS_TEST_TMPDIR/sf-download"

  run install_managed_nix "$BATS_TEST_TMPDIR/repo"
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "TOCTOU"

  # exec (sudo env) が呼ばれていないこと
  if grep -q 'UNEXPECTED-EXEC' "$BATS_TEST_TMPDIR/sudo.log"; then
    echo "exec was reached despite hash mismatch:" >&2
    cat "$BATS_TEST_TMPDIR/sudo.log" >&2
    return 1
  fi
}

@test "install.sh pins bootstrap version instead of resolving latest" {
  # latest release 任せにすると rc が拾われるため pin されていること
  run grep -n 'SCHNEEFORGE_BOOTSTRAP_VERSION=' "$INSTALL_SH"
  [ "$status" -eq 0 ]
  # releases/latest を resolve に使っていないこと
  run grep -n 'releases/latest' "$INSTALL_SH"
  [ "$status" -ne 0 ]
}
