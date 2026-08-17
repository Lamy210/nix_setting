#!/usr/bin/env bash
set -eu

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_DIR"

# 共通関数を source
# shellcheck source=scripts/resolve-tools.sh
. "$REPO_DIR/scripts/resolve-tools.sh"

if ! resolve_nix; then
  echo "Nix is not installed."
  echo
  echo "Install Nix first:"
  echo "  curl -L https://nixos.org/nix/install | sh"
  exit 1
fi

if ! resolve_git; then
  echo "Git is not installed."
  echo
  echo "Install Git first via your OS package manager."
  exit 1
fi

detect_host() {
  local arch
  case "$(uname -s)" in
  Darwin)
    arch="$(uname -m)"
    case "$arch" in
    arm64 | aarch64) echo "darwin-aarch64" ;;
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
darwin-aarch64 | linux | linux-arm)
  echo "Detected host: $HOST"
  ;;
*)
  echo "Unsupported platform: $(uname -s) $(uname -m)"
  exit 1
  ;;
esac

echo
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/schneeforge"
mkdir -p "$STATE_DIR"
USERNAME="$(whoami)"
if [ -z "$USERNAME" ]; then
  echo "Could not determine username" >&2
  exit 1
fi
case "$(uname -s)" in
Darwin) USER_HOME="/Users/$USERNAME" ;;
*) USER_HOME="/home/$USERNAME" ;;
esac
MACHINE_INPUT="$STATE_DIR/machine.nix"
cat >"$MACHINE_INPUT" <<EOF
{
  username = "$USERNAME";
  homeDirectory = "$USER_HOME";
  hostname = "$(hostname)";
}
EOF
echo "Generated machine input: $MACHINE_INPUT"
MACHINE_OVERRIDE=(--override-input machine "$MACHINE_INPUT")

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
if [ "$HOST" = "darwin-aarch64" ]; then
  echo "Applying nix-darwin + home-manager ($HOST)..."
  "$NIX_BIN" run --inputs-from . "${MACHINE_OVERRIDE[@]}" nix-darwin#darwin-rebuild -- switch --flake ".#$HOST"
else
  echo "Building home-manager generation ($HOST)..."
  "$NIX_BIN" build "${MACHINE_OVERRIDE[@]}" ".#homeConfigurations.${HOST}.activationPackage" --out-link ./result
  echo "Activating..."
  ./result/activate
fi

echo
echo "Done. Reload WezTerm with Ctrl+Shift+R"
