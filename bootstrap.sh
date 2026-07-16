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
  cat >> "$HOME/.config/nix/nix.conf" <<'NIXCONF'
experimental-features = nix-command flakes
NIXCONF
fi

SYSTEM="$(nix eval --impure --raw --expr 'builtins.currentSystem')"
USERNAME="$(whoami)"
HOME_DIR="$HOME"

cat > user-options.nix <<OPTIONS
{
  username = "${USERNAME}";
  homeDirectory = "${HOME_DIR}";
  system = "${SYSTEM}";
}
OPTIONS

echo "Generated user-options.nix:"
cat user-options.nix

echo
echo "Running home-manager switch..."
nix run github:nix-community/home-manager -- switch -b hm-bak --flake .#default
