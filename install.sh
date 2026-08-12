#!/usr/bin/env bash
set -eu

REPO_URL="https://github.com/Lamy210/nix_setting.git"
REPO_DIR="${NIX_SETTING_DIR:-$HOME/nix_setting}"

echo "=== nix_setting installer ==="
echo

# 1. Check Nix
if ! command -v nix >/dev/null 2>&1; then
  echo "[1/4] Nix not found. Installing..."
  curl -L https://nixos.org/nix/install | sh
  # shellcheck disable=SC1091
  . "$HOME/.nix-profile/etc/profile.d/nix.sh" 2>/dev/null || true
else
  echo "[1/4] Nix: $(nix --version)"
fi

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

# 3. Clone repository
if [ -d "$REPO_DIR/.git" ]; then
  echo "[3/4] Repository exists: $REPO_DIR"
else
  echo "[3/4] Cloning $REPO_URL ..."
  git clone "$REPO_URL" "$REPO_DIR"
fi

# 4. Bootstrap (detect host + build + apply)
echo "[4/4] Applying configuration..."
(
  cd "$REPO_DIR"
  ./bootstrap.sh
)

echo
echo "Done. Reload your shell or terminal."
