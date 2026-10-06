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

resolve_schneeforge_state_dir() {
  local state_home
  if [ -n "${XDG_STATE_HOME:-}" ] && [ "${XDG_STATE_HOME#/}" != "$XDG_STATE_HOME" ]; then
    state_home="$XDG_STATE_HOME"
  else
    state_home="${HOME:?HOME must be set}/.local/state"
  fi
  printf '%s/schneeforge\n' "$state_home"
}

resolve_machine_home() {
  if [ -z "${HOME:-}" ]; then
    echo "Could not determine home directory" >&2
    return 1
  fi
  printf '%s\n' "$HOME"
}

# Environment-derived machine facts are emitted inside Nix double-quoted strings.
# Keep shell bootstrap escaping in parity with core MachineFacts::to_machine_nix.
escape_nix_string() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//\$\{/\\\$\{}"
  printf '%s\n' "$value"
}

# nix.conf の存在や文字列ではなく、resolved Nix が実際に認識している
# experimental-features を確認する。
nix_has_required_flake_features() {
  local features
  if ! features="$("$NIX_BIN" config show experimental-features 2>/dev/null)"; then
    return 2
  fi
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
  local feature_status=0
  if nix_has_required_flake_features; then
    return 0
  else
    feature_status=$?
  fi
  if [ "$feature_status" -eq 2 ]; then
    echo "Failed to inspect effective Nix settings; refusing to modify Nix config" >&2
    return 1
  fi

  if [ -n "${XDG_CONFIG_HOME:-}" ] && [ "${XDG_CONFIG_HOME#/}" != "$XDG_CONFIG_HOME" ]; then
    config_home="$XDG_CONFIG_HOME"
  elif [ -n "${HOME:-}" ] && [ "${HOME#/}" != "$HOME" ]; then
    config_home="$HOME/.config"
  else
    echo "Cannot enable flakes: XDG_CONFIG_HOME or HOME must be set to an absolute path" >&2
    return 1
  fi
  conf="$config_home/nix/nix.conf"

  mkdir -p "$(dirname "$conf")"
  # 既存 nix.conf が末尾改行なしでも設定行を連結しない。
  if [ -s "$conf" ]; then
    printf '\n' >>"$conf"
  fi
  printf '%s\n' 'extra-experimental-features = nix-command flakes' >>"$conf"

  if nix_has_required_flake_features; then
    return 0
  else
    feature_status=$?
  fi
  if [ "$feature_status" -eq 2 ]; then
    echo "Failed to verify effective Nix settings after updating $conf" >&2
  else
    echo "Failed to enable flakes: $NIX_BIN config show experimental-features still lacks nix-command / flakes" >&2
  fi
  return 1
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
STATE_DIR="$(resolve_schneeforge_state_dir)"
mkdir -p "$STATE_DIR"
USERNAME="$(whoami)"
if [ -z "$USERNAME" ]; then
  echo "Could not determine username" >&2
  exit 1
fi
USER_HOME="$(resolve_machine_home)"
MACHINE_USERNAME="$(escape_nix_string "$USERNAME")"
MACHINE_HOME="$(escape_nix_string "$USER_HOME")"
MACHINE_HOSTNAME="$(escape_nix_string "$(hostname)")"
MACHINE_INPUT="$STATE_DIR/machine.nix"
cat >"$MACHINE_INPUT" <<EOF
{
  username = "$MACHINE_USERNAME";
  homeDirectory = "$MACHINE_HOME";
  hostname = "$MACHINE_HOSTNAME";
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
