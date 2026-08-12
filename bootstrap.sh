#!/usr/bin/env bash
set -eu

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_DIR"

if ! command -v nix >/dev/null 2>&1; then
  echo "Nix is not installed."
  echo
  echo "Install Nix first:"
  echo "  curl -L https://nixos.org/nix/install | sh"
  exit 1
fi

mkdir -p "$HOME/.config/nix"

if ! grep -q "experimental-features" "$HOME/.config/nix/nix.conf" 2>/dev/null; then
  cat >>"$HOME/.config/nix/nix.conf" <<'NIXCONF'
experimental-features = nix-command flakes
NIXCONF
fi

echo "Building home-manager generation..."
nix build .#homeConfigurations.default.activationPackage --out-link ./result

echo
echo "Backing up existing dotfiles..."
BACKUP_DIR="$HOME/hm-bak-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"
for f in .zshrc .zprofile .gitconfig .config/starship.toml .config/wezterm/wezterm.lua .config/atuin/config.toml .config/openspec/config.json .config/mise/config.toml; do
  [ -f "$HOME/$f" ] && cp "$HOME/$f" "$BACKUP_DIR/$(echo $f | tr '/' '_')"
done
echo "Backed up to $BACKUP_DIR"

echo
echo "Activating..."
./result/activate

echo
echo "Done. Reload WezTerm with Ctrl+Shift+R"
