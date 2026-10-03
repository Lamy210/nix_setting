#!/usr/bin/env bats

# resolve-tools.sh の resolve_tool / resolve_nix 関数を検証する。
# Rust 側 tool.rs のテストと同じ不変条件を shell 側でも保証する。

setup() {
  # テスト用一領域
  TMPDIR_TEST="$(mktemp -d)"
  # PATH と既知 env を固定
  unset SCHNEEFORGE_NIX_BIN || true
  export PATH="/usr/bin:/bin"
  export HOME="$TMPDIR_TEST/home"
  export USER="testuser"
  mkdir -p "$HOME"
  # shellcheck source=../scripts/resolve-tools.sh
  . "$BATS_TEST_DIRNAME/../scripts/resolve-tools.sh"
}

teardown() {
  rm -rf "$TMPDIR_TEST"
}

@test "resolve_tool finds binary via env override" {
  mkdir -p "$TMPDIR_TEST/custom/bin"
  cat >"$TMPDIR_TEST/custom/bin/mytool" <<'EOF'
#!/bin/sh
exit 0
EOF
  chmod +x "$TMPDIR_TEST/custom/bin/mytool"
  export SCHNEEFORGE_MYTOOL_BIN="$TMPDIR_TEST/custom/bin/mytool"
  resolve_tool "mytool"
  [ "$MYTOOL_BIN" = "$TMPDIR_TEST/custom/bin/mytool" ]
}

@test "resolve_tool canonicalizes relative env override to absolute path" {
  mkdir -p "$TMPDIR_TEST/relative-env/bin"
  cat >"$TMPDIR_TEST/relative-env/bin/mytool_relative_env" <<'EOF'
#!/bin/sh
exit 0
EOF
  chmod +x "$TMPDIR_TEST/relative-env/bin/mytool_relative_env"
  cd "$TMPDIR_TEST"
  export SCHNEEFORGE_MYTOOL_RELATIVE_ENV_BIN="relative-env/bin/mytool_relative_env"

  resolve_tool "mytool_relative_env"

  [ "$MYTOOL_RELATIVE_ENV_BIN" = "$TMPDIR_TEST/relative-env/bin/mytool_relative_env" ]
}

@test "resolve_tool propagates env override canonicalization failure" {
  mkdir -p "$TMPDIR_TEST/canonicalize-env/bin"
  cat >"$TMPDIR_TEST/canonicalize-env/bin/mytool_canonicalize_env" <<'EOF'
#!/bin/sh
exit 0
EOF
  chmod +x "$TMPDIR_TEST/canonicalize-env/bin/mytool_canonicalize_env"
  export SCHNEEFORGE_MYTOOL_CANONICALIZE_ENV_BIN="$TMPDIR_TEST/canonicalize-env/bin/mytool_canonicalize_env"
  canonicalize_executable() { return 1; }

  run resolve_tool "mytool_canonicalize_env"

  [ "$status" -ne 0 ]
}

@test "resolve_tool finds binary via PATH" {
  mkdir -p "$TMPDIR_TEST/pathdir"
  cat >"$TMPDIR_TEST/pathdir/mytool2" <<'EOF'
#!/bin/sh
exit 0
EOF
  chmod +x "$TMPDIR_TEST/pathdir/mytool2"
  export PATH="$TMPDIR_TEST/pathdir:/usr/bin:/bin"
  resolve_tool "mytool2"
  [ "$MYTOOL2_BIN" = "$TMPDIR_TEST/pathdir/mytool2" ]
}

@test "resolve_tool canonicalizes relative PATH result to absolute path" {
  mkdir -p "$TMPDIR_TEST/relative-path"
  cat >"$TMPDIR_TEST/relative-path/mytool_relative_path" <<'EOF'
#!/bin/sh
exit 0
EOF
  chmod +x "$TMPDIR_TEST/relative-path/mytool_relative_path"
  cd "$TMPDIR_TEST"
  export PATH="relative-path:/usr/bin:/bin"

  resolve_tool "mytool_relative_path"

  [ "$MYTOOL_RELATIVE_PATH_BIN" = "$TMPDIR_TEST/relative-path/mytool_relative_path" ]
}

@test "resolve_tool propagates PATH canonicalization failure" {
  mkdir -p "$TMPDIR_TEST/canonicalize-path/bin"
  cat >"$TMPDIR_TEST/canonicalize-path/bin/mytool_canonicalize_path" <<'EOF'
#!/bin/sh
exit 0
EOF
  chmod +x "$TMPDIR_TEST/canonicalize-path/bin/mytool_canonicalize_path"
  export PATH="$TMPDIR_TEST/canonicalize-path/bin:/usr/bin:/bin"
  canonicalize_executable() { return 1; }

  run resolve_tool "mytool_canonicalize_path"

  [ "$status" -ne 0 ]
}

@test "resolve_tool canonicalizes symlink override like Rust resolver" {
  mkdir -p "$TMPDIR_TEST/real/bin" "$TMPDIR_TEST/link/bin"
  cat >"$TMPDIR_TEST/real/bin/mytool_symlink" <<'EOF'
#!/bin/sh
exit 0
EOF
  chmod +x "$TMPDIR_TEST/real/bin/mytool_symlink"
  ln -s "$TMPDIR_TEST/real/bin/mytool_symlink" "$TMPDIR_TEST/link/bin/mytool_symlink"
  export SCHNEEFORGE_MYTOOL_SYMLINK_BIN="$TMPDIR_TEST/link/bin/mytool_symlink"

  resolve_tool "mytool_symlink"

  [ "$MYTOOL_SYMLINK_BIN" = "$TMPDIR_TEST/real/bin/mytool_symlink" ]
}

@test "env override beats PATH" {
  mkdir -p "$TMPDIR_TEST/pathdir" "$TMPDIR_TEST/envdir"
  for d in pathdir envdir; do
    cat >"$TMPDIR_TEST/$d/mytool3" <<'EOF'
#!/bin/sh
exit 0
EOF
    chmod +x "$TMPDIR_TEST/$d/mytool3"
  done
  export PATH="$TMPDIR_TEST/pathdir:/usr/bin:/bin"
  export SCHNEEFORGE_MYTOOL3_BIN="$TMPDIR_TEST/envdir/mytool3"
  resolve_tool "mytool3"
  [ "$MYTOOL3_BIN" = "$TMPDIR_TEST/envdir/mytool3" ]
}

@test "resolve_tool finds binary in XDG_STATE_HOME nix profile" {
  mkdir -p "$TMPDIR_TEST/xdg/nix/profile/bin"
  cat >"$TMPDIR_TEST/xdg/nix/profile/bin/mytool4" <<'EOF'
#!/bin/sh
exit 0
EOF
  chmod +x "$TMPDIR_TEST/xdg/nix/profile/bin/mytool4"
  export XDG_STATE_HOME="$TMPDIR_TEST/xdg"
  # PATH に無いツールでも XDG state から発見できる
  export PATH="/usr/bin:/bin"
  resolve_tool "mytool4"
  [ "$MYTOOL4_BIN" = "$TMPDIR_TEST/xdg/nix/profile/bin/mytool4" ]
}

@test "resolve_tool propagates known-path canonicalization failure" {
  mkdir -p "$HOME/.nix-profile/bin"
  cat >"$HOME/.nix-profile/bin/mytool_canonicalize_known" <<'EOF'
#!/bin/sh
exit 0
EOF
  chmod +x "$HOME/.nix-profile/bin/mytool_canonicalize_known"
  export PATH="/usr/bin:/bin"
  canonicalize_executable() { return 1; }

  run resolve_tool "mytool_canonicalize_known"

  [ "$status" -ne 0 ]
}

@test "resolve_tool finds binary in system profile" {
  # /nix/var/nix/profiles/default/bin を模擬するのは root 権限が必要なので、
  # 代わりに default candidate である /opt/homebrew/bin を検証対象外とし、
  # ~/nix-profile で代用
  mkdir -p "$HOME/.nix-profile/bin"
  cat >"$HOME/.nix-profile/bin/mytool5" <<'EOF'
#!/bin/sh
exit 0
EOF
  chmod +x "$HOME/.nix-profile/bin/mytool5"
  export PATH="/usr/bin:/bin"
  resolve_tool "mytool5"
  [ "$MYTOOL5_BIN" = "$HOME/.nix-profile/bin/mytool5" ]
}

@test "resolve_tool returns non-zero when not found" {
  unset SCHNEEFORGE_NOTOOL_BIN || true
  export PATH="/usr/bin:/bin"
  run resolve_tool "definitely_no_such_tool_xyz"
  [ "$status" -ne 0 ]
}

@test "resolve_nix exports NIX_BIN" {
  mkdir -p "$HOME/.nix-profile/bin"
  cat >"$HOME/.nix-profile/bin/nix" <<'EOF'
#!/bin/sh
exit 0
EOF
  chmod +x "$HOME/.nix-profile/bin/nix"
  export PATH="/usr/bin:/bin"
  resolve_nix
  [ -n "$NIX_BIN" ]
  [ -x "$NIX_BIN" ]
}

@test "is_executable rejects non-executable file" {
  echo "not executable" >"$TMPDIR_TEST/plainfile"
  chmod 644 "$TMPDIR_TEST/plainfile"
  run is_executable "$TMPDIR_TEST/plainfile"
  [ "$status" -ne 0 ]
}

@test "is_executable rejects nonexistent path" {
  run is_executable "$TMPDIR_TEST/__no_such_file__"
  [ "$status" -ne 0 ]
}

@test "ensure_nix_state_dir creates XDG state directory" {
  export XDG_STATE_HOME="$TMPDIR_TEST/xdg"
  ensure_nix_state_dir
  [ -d "$TMPDIR_TEST/xdg/nix/profiles" ]
}

@test "ensure_nix_state_dir creates default state directory" {
  unset XDG_STATE_HOME || true
  ensure_nix_state_dir
  [ -d "$HOME/.local/state/nix/profiles" ]
}
