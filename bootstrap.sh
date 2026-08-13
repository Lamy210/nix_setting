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

detect_host() {
  local arch
  case "$(uname -s)" in
  Darwin)
    arch="$(uname -m)"
    case "$arch" in
    arm64 | aarch64) echo "macbook-air" ;;
    *) echo "unsupported" ;;
    esac
    ;;
  Linux)
    arch="$(uname -m)"
    case "$arch" in
    aarch64 | arm64) echo "linux-arm" ;;
    x86_64 | amd64) echo "linux" ;;
    *) echo "unsupported" ;;
    esac
    ;;
  *)
    echo "unsupported"
    ;;
  esac
}

HOST="$(detect_host)"

case "$HOST" in
macbook-air | linux | linux-arm)
  echo "Detected host: $HOST"
  ;;
*)
  echo "Unsupported platform: $(uname -s) $(uname -m)"
  exit 1
  ;;
esac

echo
USERNAME="$(whoami)"
if [ -z "$USERNAME" ]; then
  echo "Could not determine username" >&2
  exit 1
fi
if grep -qF "username = \"$USERNAME\"" "config.toml" 2>/dev/null; then
  echo "config.toml already personalized for $USERNAME"
else
  echo "Personalizing config.toml..."
  cat >"config.toml" <<EOF
# nix_setting manifest (schema version 1)
schema = 1

[user]
username = "$USERNAME"
EOF
fi

mkdir -p "$HOME/.config/nix"

if ! grep -q "experimental-features" "$HOME/.config/nix/nix.conf" 2>/dev/null; then
  cat >>"$HOME/.config/nix/nix.conf" <<'NIXCONF'
experimental-features = nix-command flakes
NIXCONF
fi

echo
echo "Backing up existing dotfiles..."
BACKUP_DIR="$HOME/hm-bak-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"
for f in .zshrc .zprofile .gitconfig .config/starship.toml .config/wezterm/wezterm.lua .config/atuin/config.toml .config/openspec/config.json .config/mise/config.toml; do
  [ -f "$HOME/$f" ] && cp "$HOME/$f" "$BACKUP_DIR/$(echo $f | tr '/' '_')"
done
echo "Backed up to $BACKUP_DIR"

echo
if [ "$HOST" = "macbook-air" ]; then
  echo "Applying nix-darwin + home-manager ($HOST)..."
  nix run --inputs-from . nix-darwin#darwin-rebuild -- switch --flake ".#$HOST"
else
  echo "Building home-manager generation ($HOST)..."
  nix build ".#homeConfigurations.${HOST}.activationPackage" --out-link ./result
  echo "Activating..."
  ./result/activate
fi

echo
echo "Done. Reload WezTerm with Ctrl+Shift+R"
