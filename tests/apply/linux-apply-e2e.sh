#!/usr/bin/env bash
# Actual Home Manager application on disposable GitHub-hosted Linux only.
set -euo pipefail

if [[ ${GITHUB_ACTIONS:-} != true || ${RUNNER_ENVIRONMENT:-} != github-hosted || ${CI:-} != true ]]; then
  echo "ERROR: real apply E2E requires an ephemeral GitHub-hosted runner" >&2
  exit 2
fi
if [[ "$(id -u)" == 0 || "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
  echo "ERROR: unsupported real-apply execution host" >&2
  exit 2
fi
if [[ -z ${GITHUB_WORKSPACE:-} || "$(pwd -P)" != "$(realpath "$GITHUB_WORKSPACE")" ]]; then
  echo "ERROR: run only from GITHUB_WORKSPACE" >&2
  exit 2
fi

export NIX_SETTING_DIR="$GITHUB_WORKSPACE"

SF="$PWD/target/release/schneeforge"
test -x "$SF"
command -v nix >/dev/null
command -v git >/dev/null

echo "=== Initial state ==="
"$SF" --version
nix --version
"$SF" profile set minimal
"$SF" profile show
"$SF" plan

# Home Manager links ~/.gitconfig to the read-only Nix store. The checkout
# post-job hook copies that file into a temporary git config and then tries to
# add safe.directory. Preserve the runner's original file for post-job cleanup.
# This is safe only on the guarded, disposable GitHub-hosted runner.
git_config_backup="$(mktemp "${RUNNER_TEMP:-/tmp}/schneeforge-gitconfig.XXXXXXXX")"
git_config_existed=false
if [[ -e "$HOME/.gitconfig" ]]; then
  cp -L -- "$HOME/.gitconfig" "$git_config_backup"
  git_config_existed=true
fi

restore_runner_git_config() {
  local prior_status=$?
  trap - EXIT
  if [[ -e "$HOME/.gitconfig" || -L "$HOME/.gitconfig" ]]; then
    rm -f -- "$HOME/.gitconfig" || return 1
  fi
  if [[ $git_config_existed == true ]]; then
    install -m 600 -- "$git_config_backup" "$HOME/.gitconfig" || return 1
  fi
  rm -f -- "$git_config_backup"
  echo "Restored runner .gitconfig for checkout post-job cleanup"
  return "$prior_status"
}
trap restore_runner_git_config EXIT

echo "=== Real minimal Home Manager apply ==="
"$SF" apply

echo "=== Actual post-apply files and commands ==="
test -s "$HOME/.zshrc"
test -s "$HOME/.config/starship.toml"
test -s "$HOME/.config/atuin/config.toml"
test -s "$HOME/.gitconfig"
cmp "$HOME/.gitconfig" "$GITHUB_WORKSPACE/config/git/gitconfig"
export PATH="$HOME/.nix-profile/bin:$HOME/.local/state/nix/profile/bin:$PATH"
command -v zsh
zsh --version
command -v git
git --version
command -v starship
starship --version
"$SF" verify
"$SF" status

echo "=== Repeat apply, proving no degradation ==="
before="$(sha256sum "$HOME/.gitconfig" | cut -d ' ' -f 1)"
"$SF" apply
after="$(sha256sum "$HOME/.gitconfig" | cut -d ' ' -f 1)"
[[ $before == "$after" ]] || {
  echo "ERROR: config drift on repeat apply" >&2
  exit 1
}
"$SF" verify
test -s "$HOME/.zshrc"
echo "PASS: actual minimal profile applied, verified, and reapplied on disposable Linux runner"
