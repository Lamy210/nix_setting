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
  echo "Install Nix through SchneeForge Managed Nix first:"
  echo "  schneeforge nix install"
  echo "If the SchneeForge CLI is not installed yet, run:"
  echo "  ./install.sh"
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

# nix.conf の存在や文字列ではなく、resolved Nix が実際に認識している
# experimental-features を確認する。
nix_has_required_flake_features() {
  local features
  features="$("$NIX_BIN" config show experimental-features 2>/dev/null)" || return 1
  printf '%s\n' "$features" | awk '
    {
      for (i = 1; i <= NF; i++) {
        if ($i == "nix-command") have_nix_command = 1
        if ($i == "flakes") have_flakes = 1
      }
    }
    END { exit !(have_nix_command && have_flakes) }
  '
}

ensure_flakes_enabled() {
  local config_home conf
  config_home="${XDG_CONFIG_HOME:-${HOME:?HOME must be set}/.config}"
  conf="$config_home/nix/nix.conf"

  if nix_has_required_flake_features; then
    return 0
  fi

  mkdir -p "$(dirname "$conf")"
  printf '%s\n' 'experimental-features = nix-command flakes' >>"$conf"

  if ! nix_has_required_flake_features; then
    echo "Failed to enable flakes: nix config show experimental-features still lacks nix-command / flakes" >&2
    return 1
  fi
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

ensure_flakes_enabled

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
