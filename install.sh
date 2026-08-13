#!/usr/bin/env bash
set -eu

REPO_URL="${SCHNEEFORGE_REPO_URL:-https://github.com/Lamy210/nix_setting.git}"
REPO_DIR="${NIX_SETTING_DIR:-$HOME/nix_setting}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=scripts/resolve-tools.sh
. "$SCRIPT_DIR/scripts/resolve-tools.sh"

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
echo "[4/4] Applying configuration..."
(
  cd "$REPO_DIR"
  ./bootstrap.sh
)

echo
echo "Done. Reload your shell or terminal."
