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
# --- end inline resolver ---

echo "=== nix_setting installer ==="
echo

# 1. Check Nix (resolve_nix で PATH + 既知パス両方を探索)
if ! resolve_nix; then
  echo "[1/4] Nix not found. Installing..."
  curl -L https://nixos.org/nix/install | sh
  # shellcheck disable=SC1091
  . "$HOME/.nix-profile/etc/profile.d/nix.sh" 2>/dev/null || true
  # 再解決を試みる（インストール直後は PATH に無いことがある）
  if ! resolve_nix; then
    echo "[error] Nix をインストールしたが解決できない。シェルを再起動して再実行してください。" >&2
    exit 1
  fi
  echo "[1/4] Nix: $NIX_BIN"
else
  echo "[1/4] Nix found: $NIX_BIN"
fi

# Nix installer が作らないことがあるフォルダを保証
ensure_nix_state_dir

# 2. Enable flakes
mkdir -p "$HOME/.config/nix"
if ! grep -q "experimental-features" "$HOME/.config/nix/nix.conf" 2>/dev/null; then
  echo "[2/4] Enabling flakes..."
  cat >>"$HOME/.config/nix/nix.conf" <<'NIXCONF'
experimental-features = nix-command flakes
NIXCONF
else
  echo "[2/4] Flakes: enabled"
fi

# 3. Clone repository (resolve_git で絶対パス取得)
if ! resolve_git; then
  echo "[3/4] Git not found. Install Git first via your OS package manager." >&2
  exit 1
fi
if [ -d "$REPO_DIR/.git" ]; then
  echo "[3/4] Repository exists: $REPO_DIR"
else
  echo "[3/4] Cloning $REPO_URL ..."
  "$GIT_BIN" clone "$REPO_URL" "$REPO_DIR"
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
