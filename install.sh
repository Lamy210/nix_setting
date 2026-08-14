#!/usr/bin/env bash
#
# SchneeForge installer. `curl|bash` で実行されるため、リポジトリ内の
# scripts/resolve-tools.sh を source するのではなく、必要最小限の resolver を
# inline で持つ。リポジトリ clone 後に bootstrap.sh が起動する際は、リポジトリ側の
# scripts/resolve-tools.sh が使われる (関数名・挙動は inline 版と同一)。
set -euo pipefail

REPO_URL="${SCHNEEFORGE_REPO_URL:-https://github.com/Lamy210/nix_setting.git}"
REPO_DIR="${NIX_SETTING_DIR:-$HOME/nix_setting}"

# --- inline minimal tool resolver (clone 前に動くよう install.sh 単独で解決可能) ---
# 探索順は scripts/resolve-tools.sh と一致 (Rust tool.rs とも同期)
is_executable() {
  [ -f "$1" ] && [ -x "$1" ]
}

# resolve_tool NAME -- <NAME>_BIN 絶対パスを export。見つからなければ 1
resolve_tool() {
  local name="$1"
  local upper
  upper="$(printf '%s' "$name" | tr '[:lower:]-' '[:upper:]_')"
  local env_var="SCHNEEFORGE_${upper}_BIN"
  local out_var="${upper}_BIN"

  # 1. env override
  if [ -n "${!env_var:-}" ] && is_executable "${!env_var}"; then
    export "${out_var}=${!env_var}"
    return 0
  fi

  # 2. PATH
  if command -v "$name" >/dev/null 2>&1; then
    local p
    p="$(command -v "$name")"
    if is_executable "$p"; then
      export "${out_var}=$p"
      return 0
    fi
  fi

  # 候補ディレクトリ (3-10)
  local candidates=()
  if [ -n "${XDG_STATE_HOME:-}" ]; then
    candidates+=("${XDG_STATE_HOME}/nix/profile/bin")
  fi
  if [ -n "${HOME:-}" ]; then
    candidates+=("${HOME}/.local/state/nix/profile/bin")
  fi
  if [ -n "${NIX_PROFILE:-}" ]; then
    candidates+=("${NIX_PROFILE}/bin")
  fi
  if [ -n "${HOME:-}" ]; then
    candidates+=("${HOME}/.nix-profile/bin")
  fi
  if [ -n "${USER:-}" ]; then
    candidates+=("/etc/profiles/per-user/${USER}/bin")
  fi
  candidates+=("/nix/var/nix/profiles/default/bin")
  candidates+=("/opt/homebrew/bin")
  candidates+=("/usr/local/bin")

  for dir in "${candidates[@]}"; do
    local candidate="${dir}/${name}"
    if is_executable "$candidate"; then
      export "${out_var}=$candidate"
      return 0
    fi
  done

  return 1
}

resolve_nix() { resolve_tool "nix"; }
resolve_git() { resolve_tool "git"; }

# Nix installer が作らないことがある state dir を保証
ensure_nix_state_dir() {
  local state_dir
  if [ -n "${XDG_STATE_HOME:-}" ]; then
    state_dir="${XDG_STATE_HOME}/nix/profiles"
  else
    state_dir="${HOME:?HOME must be set}/.local/state/nix/profiles"
  fi
  [ -d "$state_dir" ] || mkdir -p "$state_dir"
}

# Managed Nix 経路で使う schneeforge CLI binary を GitHub Release から取得し、
# CHECKSUMS.txt の sha256 と突合してから path を標準出力へ出す。
# (version pinning / 検証の詳細は schneeforge nix install 側が持つため、ここでは
#  download 元の integrity のみ確認する)
fetch_schneeforge_binary() {
  local os arch asset_base url_base version tmp_dir expect actual
  case "$(uname -s)" in
    Linux) os="linux" ;;
    Darwin) os="darwin" ;;
    *)
      echo "[error] unsupported OS: $(uname -s)" >&2
      return 1
      ;;
  esac
  case "$(uname -m)" in
    x86_64 | amd64) arch="x86_64" ;;
    aarch64 | arm64) arch="aarch64" ;;
    *)
      echo "[error] unsupported arch: $(uname -m)" >&2
      return 1
      ;;
  esac
  if [ "$os" = "darwin" ]; then
    # darwin release binary は aarch64 のみ
    asset_base="schneeforge-aarch64-darwin"
  else
    asset_base="schneeforge-${arch}-${os}"
  fi

  version="${SCHNEEFORGE_VERSION:-}"
  if [ -z "$version" ]; then
    version="$(curl -fsSL "https://api.github.com/repos/Lamy210/nix_setting/releases/latest" |
      sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)"
  fi
  if [ -z "$version" ]; then
    echo "[error] latest release を取得できません。" >&2
    return 1
  fi
  url_base="https://github.com/Lamy210/nix_setting/releases/download/${version}"

  tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/schneeforge-install.XXXXXX")"
  curl -fsSL -o "$tmp_dir/CHECKSUMS.txt" "$url_base/CHECKSUMS.txt" || {
    echo "[error] CHECKSUMS.txt の download に失敗: $version" >&2
    return 1
  }
  # asset 名 (path) は release workflow の配置に依存するため前方一致で取り出す
  expect="$(sed -n "s|^[0-9a-f]\{64\}  .*/${asset_base}\$|&|p" "$tmp_dir/CHECKSUMS.txt" | awk '{print $1}')"
  if [ -z "$expect" ]; then
    echo "[error] CHECKSUMS.txt に ${asset_base} の entry が無い: $version" >&2
    return 1
  fi

  curl -fsSL -o "$tmp_dir/$asset_base" "$url_base/$asset_base" || {
    echo "[error] ${asset_base} の download に失敗: $version" >&2
    return 1
  }
  actual="$(sha256sum "$tmp_dir/$asset_base" | awk '{print $1}')"
  if [ "$actual" != "$expect" ]; then
    echo "[error] sha256 mismatch (${asset_base}):" >&2
    echo "  expect: $expect" >&2
    echo "  actual: $actual" >&2
    return 1
  fi
  chmod +x "$tmp_dir/$asset_base"
  echo "$tmp_dir/$asset_base"
}

# Managed Nix (schneeforge nix install) を実行する。
# root 権限が必要なため sudo で再実行する。CLI の D8 最終確認は stdin が
# TTY でないと fail-closed されるため、curl|bash で stdin を奪われている
# 場合は /dev/tty を繋いで確認可能にする。
install_managed_nix() {
  local repo_dir="$1" sf_bin
  sf_bin="$(fetch_schneeforge_binary)" || return 1
  echo "[nix] Managed Nix install を開始します (NixOS/nix-installer, version pinned)..."
  # sudo 環境でも repo 位置が分かるように NIX_SETTING_DIR を渡す
  if [ -t 0 ]; then
    sudo env NIX_SETTING_DIR="$repo_dir" "$sf_bin" nix install
  else
    # shellcheck disable=SC2024  # redirect 自体が目的 (curl|bash の stdin を外す)
    sudo env NIX_SETTING_DIR="$repo_dir" "$sf_bin" nix install </dev/tty
  fi
  local rc=$?
  rm -f "$sf_bin"
  rmdir "$(dirname "$sf_bin")" 2>/dev/null || true
  if [ $rc -ne 0 ]; then
    echo "[error] Managed Nix install に失敗 (exit $rc)" >&2
    return 1
  fi
  # install 直後は login shell の PATH に /nix/... が無いことがある
  if [ -e /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh ]; then
    # shellcheck disable=SC1091
    . /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh 2>/dev/null || true
  fi
  if ! resolve_nix; then
    echo "[error] Nix install は完了したが解決できない。shell を reload して再実行してください。" >&2
    return 1
  fi
}
# --- end inline resolver ---

echo "=== nix_setting installer ==="
echo

# 1. Git (Managed Nix 経路は repo の bootstrap-manifest.toml を必要とするため、
#    Nix の前に clone を済ませる)
if ! resolve_git; then
  echo "[1/4] Git not found. Install Git first via your OS package manager." >&2
  exit 1
fi
if [ -d "$REPO_DIR/.git" ]; then
  echo "[1/4] Repository exists: $REPO_DIR"
else
  echo "[1/4] Cloning $REPO_URL ..."
  "$GIT_BIN" clone "$REPO_URL" "$REPO_DIR"
fi

# 2. Check Nix (resolve_nix で PATH + 既知パス両方を探索)。
#    未検出の場合は curl|sh ではなく SchneeForge Managed Nix で install する
#    (ownership record 付き。ADR-0001 参照)
if resolve_nix; then
  echo "[2/4] Nix found: $NIX_BIN"
else
  echo "[2/4] Nix not found. Installing via SchneeForge Managed Nix..."
  install_managed_nix "$REPO_DIR" || exit 1
  echo "[2/4] Nix: $NIX_BIN"
fi

# Nix installer が作らないことがあるフォルダを保証
ensure_nix_state_dir

# 3. Enable flakes
mkdir -p "$HOME/.config/nix"
if ! grep -q "experimental-features" "$HOME/.config/nix/nix.conf" 2>/dev/null; then
  echo "[3/4] Enabling flakes..."
  cat >>"$HOME/.config/nix/nix.conf" <<'NIXCONF'
experimental-features = nix-command flakes
NIXCONF
else
  echo "[3/4] Flakes: enabled"
fi

# 4. Bootstrap (detect host + build + apply)
#    この時点からリポジトリ内の scripts/resolve-tools.sh が利用可能になる
echo "[4/4] Applying configuration..."
(
  cd "$REPO_DIR"
  ./bootstrap.sh
)

echo
echo "Done. Reload your shell or terminal."
