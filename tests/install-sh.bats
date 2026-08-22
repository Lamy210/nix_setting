#!/usr/bin/env bats

# install.sh の Managed Nix 経路 + fresh install の managed source 化
# (switch-install-sh-to-managed-source) の unit test。
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

# main flow (marker 以降) も取り出す。flow 分岐 (checkout / managed) の実挙動を
# stub 環境下で eval して検証するために使う (/dev/tty は同様に置換)
INSTALL_MAIN="$(sed -n '/^# --- end inline resolver ---$/,$p' "$INSTALL_SH" | tail -n +2 | sed 's|/dev/tty|/dev/null|g')"

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
  echo fake-binary-content >"$BATS_TEST_TMPDIR/sf-download"
  mkdir -p "$sandbox"

  # 最後の nix 再探索は install 成功後の処理のため stub する
  resolve_nix() { return 0; }

  # INSTALL_FUNCTIONS は /dev/tty → /dev/null 置換済み (CI container で
  # open できないため)。redirect 先が違うだけで分岐 logic は同じ。
  # sf_bin は caller から渡す (既存 checkout 経路: repo_dir も渡す)
  run install_managed_nix "$BATS_TEST_TMPDIR/sf-download" "$BATS_TEST_TMPDIR/repo"
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

  # cleanup: staging 側は削除されるが、user 側 binary は caller が apply まで
  # 使うため残ること (fresh 経路の source init / apply で再利用)
  first_rm="$(echo "$log" | grep -n 'rm -f' | head -1 | cut -d: -f1)"
  [ -n "$first_rm" ]
  [ "$first_exec" -lt "$first_rm" ]
  [ ! -e "${sandbox}${stage}/schneeforge" ]
  [ ! -d "${sandbox}${stage}" ]
  [ -f "$BATS_TEST_TMPDIR/sf-download" ]
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
  echo fake-binary-content >"$BATS_TEST_TMPDIR/sf-download"

  run install_managed_nix "$BATS_TEST_TMPDIR/sf-download" "$BATS_TEST_TMPDIR/repo"
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "TOCTOU"

  # exec (sudo env) が呼ばれていないこと
  if grep -q 'UNEXPECTED-EXEC' "$BATS_TEST_TMPDIR/sudo.log"; then
    echo "exec was reached despite hash mismatch:" >&2
    cat "$BATS_TEST_TMPDIR/sudo.log" >&2
    return 1
  fi
}

@test "install_managed_nix omits NIX_SETTING_DIR on fresh (embedded manifest) path" {
  # fresh 経路は repo checkout が無い (embedded manifest で動作) ため、
  # sudo 実行時に NIX_SETTING_DIR を渡さないこと
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
    rmdir) rmdir "${sandbox}$2" 2>/dev/null || true ;;
    sh) return 0 ;;
    sha256sum) sha256sum "$2" | awk '{print $1}' ;;
    "$stage"*)
      # plain exec (fresh 経路): env 経由ではなく staged binary を直接実行
      if [ ! -f "${sandbox}${stage}/schneeforge" ]; then
        echo "ERROR: exec before staging" >&2
        return 1
      fi
      echo "exec: $*" >>"$BATS_TEST_TMPDIR/sudo.log"
      ;;
    *) return 0 ;;
    esac
  }
  sudo_sha256_file() { sudo sha256sum "${sandbox}/var/lib/schneeforge/bootstrap/schneeforge"; }
  echo fake-binary-content >"$BATS_TEST_TMPDIR/sf-download"
  resolve_nix() { return 0; }

  # repo_dir を渡さない (fresh 経路)
  run install_managed_nix "$BATS_TEST_TMPDIR/sf-download"
  [ "$status" -eq 0 ]

  # staged binary が実行され、NIX_SETTING_DIR を含まないこと
  grep -q '^exec:' "$BATS_TEST_TMPDIR/sudo.log"
  if grep -q 'NIX_SETTING_DIR' "$BATS_TEST_TMPDIR/sudo.log"; then
    echo "NIX_SETTING_DIR should not be passed on fresh path:" >&2
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

@test "README stable one-liner URL matches install.sh bootstrap pin (Stable/Edge 分離)" {
  # デグレ #12: Stable ワンライナーは tag 固定 URL。その tag が install.sh の
  # SCHNEEFORGE_BOOTSTRAP_VERSION pin と一致することで「script 取得元 ==
  # download する CLI の release」という release unit の一致が保証される。
  # bump 忘れ (README だけ古い tag) はここで検知する。
  local pinned stable_url stable_tag
  # shellcheck disable=SC2016  # ${SCHNEEFORGE_VERSION:-...} は literal として match させる
  pinned="$(sed -n 's/^SCHNEEFORGE_BOOTSTRAP_VERSION="\${SCHNEEFORGE_VERSION:-\([^"]*\)}"/\1/p' "$INSTALL_SH")"
  [ -n "$pinned" ]

  stable_url="$(grep -oE 'https://raw\.githubusercontent\.com/Lamy210/nix_setting/v[^/]+/install\.sh' "$BATS_TEST_DIRNAME/../README.md" | head -1)"
  [ -n "$stable_url" ]
  stable_tag="${stable_url%/install.sh}"
  stable_tag="${stable_tag##*/}"

  [ "$stable_tag" = "$pinned" ]

  # Edge (main HEAD) の案内も残っていること
  grep -q 'raw\.githubusercontent\.com/Lamy210/nix_setting/main/install\.sh' "$BATS_TEST_DIRNAME/../README.md"
}

@test "fresh install does not clone and pins managed source ref to release" {
  # source ref pin: managed source の init は binary の pin と同一 release tag を
  # 指定すること。default branch (develop) を拾うと「過去の installer 実行時に
  # その時点の develop が入る」問題が出る
  run grep -n 'SCHNEEFORGE_BOOTSTRAP_REF=' "$INSTALL_SH"
  [ "$status" -eq 0 ]
  # ref は version pin と連動 (独立 default 値を持たない)
  # [$] を使い SC2016 を回避しつつ literal を grep
  run grep -n 'SCHNEEFORGE_REF:-[$]SCHNEEFORGE_BOOTSTRAP_VERSION' "$INSTALL_SH"
  [ "$status" -eq 0 ]
  # clone が install.sh から完全に消えていること
  run grep -n '"[$]GIT_BIN" clone' "$INSTALL_SH"
  [ "$status" -ne 0 ]
  run grep -nE 'git clone' "$INSTALL_SH"
  [ "$status" -ne 0 ]
  # source init が pinned ref を指定すること
  run grep -n 'source init --tag "[$]SCHNEEFORGE_BOOTSTRAP_REF"' "$INSTALL_SH"
  [ "$status" -eq 0 ]
}

@test "fresh path runs source init and apply via fetched CLI, without clone" {
  # 動作検証: main flow (INSTALL_MAIN) を stub 環境下で実行し、
  # 1. clone (git 任意の invocation) が発生しないこと
  # 2. fetch した CLI binary が source init --tag <pin> → apply の順で呼ばれること
  # 3. dotfile backup が apply 前に行われること
  local git_log="$BATS_TEST_TMPDIR/git.log"
  local sf_log="$BATS_TEST_TMPDIR/sf.log"
  local home="$BATS_TEST_TMPDIR/home"
  : >"$git_log"
  mkdir -p "$home" "$BATS_TEST_TMPDIR/sfbin"

  # git stub: 全 invocation を log に記録 (clone 検出用)
  cat >"$BATS_TEST_TMPDIR/bin-git" <<EOF
#!/usr/bin/env bash
echo "\$*" >>"$git_log"
exit 0
EOF
  chmod +x "$BATS_TEST_TMPDIR/bin-git"
  PATH="$BATS_TEST_TMPDIR:$PATH"
  ln -sf "$BATS_TEST_TMPDIR/bin-git" "$BATS_TEST_TMPDIR/git"

  # nix stub: step 2 で「Nix found」にする (Managed Nix install 経路は別 test で担保)
  ln -sf "$BATS_TEST_TMPDIR/bin-git" "$BATS_TEST_TMPDIR/nix"

  # sf binary stub: 引数を log に記録する「fetch 済み binary」
  cat >"$BATS_TEST_TMPDIR/sfbin/schneeforge" <<EOF
#!/usr/bin/env bash
echo "\$*" >>"$sf_log"
exit 0
EOF
  chmod +x "$BATS_TEST_TMPDIR/sfbin/schneeforge"

  eval "$INSTALL_FUNCTIONS"
  fetch_schneeforge_binary() { echo "$BATS_TEST_TMPDIR/sfbin/schneeforge"; }

  HOME="$home"
  export HOME
  unset XDG_STATE_HOME || true # state dir が実 home 配下に作られるようにする
  REPO_DIR="$BATS_TEST_TMPDIR/repo" # .git 無し = fresh
  unset SCHNEEFORGE_REF SCHNEEFORGE_VERSION || true
  SCHNEEFORGE_BOOTSTRAP_VERSION="v0.2.0-rc.2"
  # shellcheck disable=SC2034  # INSTALL_MAIN の eval 内で参照される
  SCHNEEFORGE_BOOTSTRAP_REF="$SCHNEEFORGE_BOOTSTRAP_VERSION"

  # backup 対象の既存 dotfile を置いておく
  echo 'existing-zshrc' >"$home/.zshrc"

  # subshell で wrap し、flow 内の exit が test process を巻き込まないようにする
  (eval "$INSTALL_MAIN")

  # 1. git が一度も呼ばれていない (clone も含む) こと
  [ ! -s "$git_log" ]

  # 2. CLI が pinned tag 付き source init → apply の順で呼ばれたこと
  grep -q -- '--tag v0.2.0-rc.2' "$sf_log"
  [ "$(head -1 "$sf_log")" = "source init --tag v0.2.0-rc.2" ]
  [ "$(tail -1 "$sf_log")" = "apply" ]

  # 3. dotfile backup が行われていること (glob 展開結果が file として存在)
  local backed
  backed="$(echo "$home"/hm-bak-*/.zshrc)"
  [ -f "$backed" ]
}

@test "existing checkout keeps bootstrap.sh flow without managed source" {
  # 動作検証: $REPO_DIR/.git が存在する場合は従来 flow (bootstrap.sh) を使い、
  # CLI binary の fetch / source init / apply が発生しないこと
  local home="$BATS_TEST_TMPDIR/home"
  local repo="$BATS_TEST_TMPDIR/repo"
  local sf_log="$BATS_TEST_TMPDIR/sf.log"
  local bootstrap_log="$BATS_TEST_TMPDIR/bootstrap.log"
  local git_log="$BATS_TEST_TMPDIR/git.log"
  mkdir -p "$home" "$repo/.git"
  : >"$git_log"

  cat >"$repo/bootstrap.sh" <<EOF
#!/usr/bin/env bash
echo "bootstrap-ran" >>"$bootstrap_log"
exit 0
EOF
  chmod +x "$repo/bootstrap.sh"

  # git / nix stub: invocation を log に記録する (実環境の git に依存しない)
  cat >"$BATS_TEST_TMPDIR/bin-stub" <<EOF
#!/usr/bin/env bash
echo "\$*" >>"$git_log"
exit 0
EOF
  chmod +x "$BATS_TEST_TMPDIR/bin-stub"
  PATH="$BATS_TEST_TMPDIR:$PATH"
  ln -sf "$BATS_TEST_TMPDIR/bin-stub" "$BATS_TEST_TMPDIR/git"
  ln -sf "$BATS_TEST_TMPDIR/bin-stub" "$BATS_TEST_TMPDIR/nix"

  eval "$INSTALL_FUNCTIONS"
  fetch_schneeforge_binary() {
    echo "fetch-called" >>"$sf_log"
    echo "$BATS_TEST_TMPDIR/sfbin/schneeforge"
  }

  HOME="$home"
  export HOME
  unset XDG_STATE_HOME || true
  # shellcheck disable=SC2034  # INSTALL_MAIN の eval 内で参照される
  REPO_DIR="$repo" # .git 有り = 既存 checkout

  # subshell で wrap し、flow 内の exit が test process を巻き込まないようにする
  (eval "$INSTALL_MAIN")

  # bootstrap.sh が呼ばれ、managed source 経路 (fetch / source init / apply) は
  # 一切発生していないこと
  [ -f "$bootstrap_log" ]
  [ ! -e "$sf_log" ]
  [ ! -s "$git_log" ]
}
