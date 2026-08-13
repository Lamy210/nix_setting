#!/usr/bin/env bash
# scripts/resolve-tools.sh
#
# SchneeForge 共通ツール解決関数。Rust 側 `crates/core/src/tool.rs` の
# `default_known_paths` と同じ探索優先度を持つ。install.sh / bootstrap.sh は
# このファイルを source して使う。
#
# 使い方:
#   source "$(dirname "$0")/scripts/resolve-tools.sh"
#   resolve_nix && echo "$NIX_BIN"
#
# Nix 2.x の XDG state 遷移を反映:
#   https://github.com/nix-community/home-manager/issues/4403
# root 以外のユーザープロファイルは $XDG_STATE_HOME/nix/profiles へ。
# なお Nix installer は同ディレクトリを作成しないことがある（要手動 mkdir）。

set -euo pipefail

# is_executable PATH -- 指定パスが実行可能ファイルか判定
is_executable() {
  [ -f "$1" ] && [ -x "$1" ]
}

# resolve_tool NAME -- ツールを解決し、<NAME>_BIN 環境変数へ絶対パスを export
#
# 探索順（Rust 側 tool.rs と一致）:
#   1. SCHNEEFORGE_<NAME>_BIN env
#   2. PATH 上の command -v
#   3. $XDG_STATE_HOME/nix/profile/bin
#   4. $HOME/.local/state/nix/profile/bin
#   5. $NIX_PROFILE/bin
#   6. $HOME/.nix-profile/bin
#   7. /etc/profiles/per-user/$USER/bin
#   8. /nix/var/nix/profiles/default/bin
#   9. /opt/homebrew/bin
#  10. /usr/local/bin
resolve_tool() {
  local name="$1"
  local upper
  upper="$(echo "$name" | tr '[:lower:]-' '[:upper:]_')"
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

  # 候補ディレクトリ（3-10）
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

resolve_nix() {
  resolve_tool "nix"
}

resolve_git() {
  resolve_tool "git"
}

resolve_brew() {
  resolve_tool "brew"
}

# ensure_nix_state_dir -- $XDG_STATE_HOME/nix/profiles が無ければ作成
# （Nix installer が自動作成しない有名な罠への対応）
ensure_nix_state_dir() {
  local state_dir
  if [ -n "${XDG_STATE_HOME:-}" ]; then
    state_dir="${XDG_STATE_HOME}/nix/profiles"
  else
    state_dir="${HOME:-$(echo ~)}/.local/state/nix/profiles"
  fi
  if [ ! -d "$state_dir" ]; then
    mkdir -p "$state_dir"
  fi
}
