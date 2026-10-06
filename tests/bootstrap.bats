#!/usr/bin/env bats

extract_detect_host() {
  sed -n '/^detect_host()/,/^}/p' "$BATS_TEST_DIRNAME/../bootstrap.sh"
}

extract_flake_functions() {
  sed -n '/^nix_has_required_flake_features()/,/^}/p' "$BATS_TEST_DIRNAME/../bootstrap.sh"
  sed -n '/^ensure_flakes_enabled()/,/^}/p' "$BATS_TEST_DIRNAME/../bootstrap.sh"
}

extract_state_dir_function() {
  sed -n '/^resolve_schneeforge_state_dir()/,/^}/p' "$BATS_TEST_DIRNAME/../bootstrap.sh"
}

extract_machine_home_function() {
  sed -n '/^resolve_machine_home()/,/^}/p' "$BATS_TEST_DIRNAME/../bootstrap.sh"
}

setup() {
  detect_host_body="$(extract_detect_host)"
  flake_functions="$(extract_flake_functions)"
  state_dir_function="$(extract_state_dir_function)"
  machine_home_function="$(extract_machine_home_function)"
}

@test "detect_host returns darwin-aarch64 on macOS arm64" {
  uname() {
    case "$1" in
      -s) echo "Darwin" ;;
      -m) echo "arm64" ;;
    esac
  }
  eval "$detect_host_body"
  result="$(detect_host)"
  [ "$result" = "darwin-aarch64" ]
}

@test "detect_host returns unsupported on macOS x86_64 (Intel Mac)" {
  uname() {
    case "$1" in
      -s) echo "Darwin" ;;
      -m) echo "x86_64" ;;
    esac
  }
  eval "$detect_host_body"
  result="$(detect_host)"
  [ "$result" = "unsupported" ]
}

@test "detect_host returns linux on Linux x86_64" {
  uname() {
    case "$1" in
      -s) echo "Linux" ;;
      -m) echo "x86_64" ;;
    esac
  }
  eval "$detect_host_body"
  result="$(detect_host)"
  [ "$result" = "linux" ]
}

@test "detect_host returns linux-arm on Linux aarch64" {
  uname() {
    case "$1" in
      -s) echo "Linux" ;;
      -m) echo "aarch64" ;;
    esac
  }
  eval "$detect_host_body"
  result="$(detect_host)"
  [ "$result" = "linux-arm" ]
}

@test "detect_host returns unsupported on unsupported OS" {
  uname() {
    case "$1" in
      -s) echo "FreeBSD" ;;
      -m) echo "x86_64" ;;
    esac
  }
  eval "$detect_host_body"
  result="$(detect_host)"
  [ "$result" = "unsupported" ]
}

@test "runtime missing-Nix guidance uses SchneeForge Managed Nix" {
  local files=(
    "$BATS_TEST_DIRNAME/../bootstrap.sh"
    "$BATS_TEST_DIRNAME/../crates/core/src/bootstrap.rs"
    "$BATS_TEST_DIRNAME/../crates/core/src/tool.rs"
    "$BATS_TEST_DIRNAME/../crates/core/src/diagnostics.rs"
    "$BATS_TEST_DIRNAME/../crates/cli/src/legacy_main.rs"
  )

  for file in "${files[@]}"; do
    ! grep -Fq "nixos.org/nix/install" "$file"
    grep -Fq "schneeforge nix install" "$file"
  done
}

@test "shell bootstrap paths verify effective flakes config instead of grepping nix.conf" {
  local files=(
    "$BATS_TEST_DIRNAME/../install.sh"
    "$BATS_TEST_DIRNAME/../bootstrap.sh"
  )

  for file in "${files[@]}"; do
    grep -Fq 'config show experimental-features' "$file"
    ! grep -Fq 'grep -q "experimental-features"' "$file"
  done
}

@test "bootstrap fails closed when effective Nix config inspection fails" {
  mkdir -p "$BATS_TEST_TMPDIR/bin"
  export XDG_CONFIG_HOME="$BATS_TEST_TMPDIR/config"
  export HOME="$BATS_TEST_TMPDIR/home"
  mkdir -p "$XDG_CONFIG_HOME/nix"
  printf '%s\n' 'experimental-features = ca-derivations' >"$XDG_CONFIG_HOME/nix/nix.conf"
  cp "$XDG_CONFIG_HOME/nix/nix.conf" "$BATS_TEST_TMPDIR/nix.conf.before"
  cat >"$BATS_TEST_TMPDIR/bin/fake-nix" <<'EOF'
#!/usr/bin/env bash
exit 23
EOF
  chmod +x "$BATS_TEST_TMPDIR/bin/fake-nix"
  export NIX_BIN="$BATS_TEST_TMPDIR/bin/fake-nix"
  eval "$flake_functions"

  run ensure_flakes_enabled
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "refusing to modify"
  cmp -s "$BATS_TEST_TMPDIR/nix.conf.before" "$XDG_CONFIG_HOME/nix/nix.conf"
}

@test "shell flakes config ignores relative XDG_CONFIG_HOME in favor of absolute HOME" {
  local scripts=(
    "$BATS_TEST_DIRNAME/../bootstrap.sh"
    "$BATS_TEST_DIRNAME/../install.sh"
  )
  local script label root functions old_pwd

  for script in "${scripts[@]}"; do
    label="$(basename "$script" .sh)"
    root="$BATS_TEST_TMPDIR/$label-relative-xdg"
    mkdir -p "$root/bin" "$root/home/.config/nix" "$root/work"
    export XDG_CONFIG_HOME="relative-config"
    export HOME="$root/home"
    export NIX_BIN="$root/bin/fake-nix"
    cat >"$NIX_BIN" <<'EOF'
#!/usr/bin/env bash
conf="${HOME}/.config/nix/nix.conf"
if grep -Fxq 'extra-experimental-features = nix-command flakes' "$conf" 2>/dev/null; then
  echo 'experimental-features = nix-command flakes'
else
  echo 'experimental-features = nix-command'
fi
EOF
    chmod +x "$NIX_BIN"
    functions="$(sed -n '/^nix_has_required_flake_features()/,/^}/p' "$script"; sed -n '/^ensure_flakes_enabled()/,/^}/p' "$script")"
    eval "$functions"

    old_pwd="$PWD"
    cd "$root/work"
    run ensure_flakes_enabled
    cd "$old_pwd"

    [ "$status" -eq 0 ]
    grep -Fxq 'extra-experimental-features = nix-command flakes' "$root/home/.config/nix/nix.conf"
    [ ! -e "$root/work/relative-config/nix/nix.conf" ]
  done
}

@test "shell flakes config fails closed when no absolute config root exists" {
  local scripts=(
    "$BATS_TEST_DIRNAME/../bootstrap.sh"
    "$BATS_TEST_DIRNAME/../install.sh"
  )
  local script label root functions old_pwd

  for script in "${scripts[@]}"; do
    label="$(basename "$script" .sh)"
    root="$BATS_TEST_TMPDIR/$label-relative-roots"
    mkdir -p "$root/bin" "$root/work"
    export XDG_CONFIG_HOME="relative-config"
    export HOME="relative-home"
    export NIX_BIN="$root/bin/fake-nix"
    cat >"$NIX_BIN" <<'EOF'
#!/usr/bin/env bash
echo 'experimental-features = nix-command'
EOF
    chmod +x "$NIX_BIN"
    functions="$(sed -n '/^nix_has_required_flake_features()/,/^}/p' "$script"; sed -n '/^ensure_flakes_enabled()/,/^}/p' "$script")"
    eval "$functions"

    old_pwd="$PWD"
    cd "$root/work"
    run ensure_flakes_enabled
    cd "$old_pwd"

    [ "$status" -ne 0 ]
    [ ! -e "$root/work/relative-config/nix/nix.conf" ]
    [ ! -e "$root/work/relative-home/.config/nix/nix.conf" ]
  done
}

@test "bootstrap state dir ignores relative XDG_STATE_HOME" {
  export XDG_STATE_HOME="relative-state"
  export HOME="$BATS_TEST_TMPDIR/home"
  mkdir -p "$HOME"
  eval "$state_dir_function"

  run resolve_schneeforge_state_dir

  [ "$status" -eq 0 ]
  [ "$output" = "$HOME/.local/state/schneeforge" ]
}

@test "bootstrap state dir honors absolute XDG_STATE_HOME" {
  export XDG_STATE_HOME="$BATS_TEST_TMPDIR/state"
  export HOME="$BATS_TEST_TMPDIR/home"
  eval "$state_dir_function"

  run resolve_schneeforge_state_dir

  [ "$status" -eq 0 ]
  [ "$output" = "$XDG_STATE_HOME/schneeforge" ]
}

@test "bootstrap state dir rejects relative HOME fallback" {
  unset XDG_STATE_HOME
  export HOME="relative-home"
  eval "$state_dir_function"

  run resolve_schneeforge_state_dir

  [ "$status" -ne 0 ]
  echo "$output" | grep -q "absolute"
  [ "$output" != "relative-home/.local/state/schneeforge" ]
}

@test "bootstrap machine home uses effective HOME" {
  export HOME="$BATS_TEST_TMPDIR/custom-home"
  eval "$machine_home_function"

  run resolve_machine_home

  [ "$status" -eq 0 ]
  [ "$output" = "$HOME" ]
}

@test "bootstrap machine home fails when HOME is unavailable" {
  unset HOME
  eval "$machine_home_function"

  run resolve_machine_home

  [ "$status" -ne 0 ]
  echo "$output" | grep -q "Could not determine home directory"
}

@test "bootstrap machine home rejects relative HOME" {
  export HOME="relative-home"
  eval "$machine_home_function"

  run resolve_machine_home

  [ "$status" -ne 0 ]
  echo "$output" | grep -q "absolute"
  [ "$output" != "relative-home" ]
}

@test "bootstrap machine input is wired to effective HOME resolver" {
  grep -Fq 'USER_HOME="$(resolve_machine_home)"' "$BATS_TEST_DIRNAME/../bootstrap.sh"
  ! grep -Fq 'USER_HOME="/Users/$USERNAME"' "$BATS_TEST_DIRNAME/../bootstrap.sh"
  ! grep -Fq 'USER_HOME="/home/$USERNAME"' "$BATS_TEST_DIRNAME/../bootstrap.sh"
}

@test "bootstrap Nix string escaping protects interpolation and delimiters" {
  local escape_function input
  escape_function="$(sed -n '/^escape_nix_string()/,/^}/p' "$BATS_TEST_DIRNAME/../bootstrap.sh")"
  [ -n "$escape_function" ]
  eval "$escape_function"

  input='user${builtins.abort "boom"}\tail'
  run escape_nix_string "$input"

  [ "$status" -eq 0 ]
  [ "$output" = 'user\${builtins.abort \"boom\"}\\tail' ]
}

@test "bootstrap machine input escapes all environment-derived strings" {
  local script="$BATS_TEST_DIRNAME/../bootstrap.sh"

  grep -Fq 'MACHINE_USERNAME="$(escape_nix_string "$USERNAME")"' "$script"
  grep -Fq 'MACHINE_HOME="$(escape_nix_string "$USER_HOME")"' "$script"
  grep -Fq 'MACHINE_HOSTNAME="$(escape_nix_string "$(hostname)")"' "$script"
  grep -Fq 'username = "$MACHINE_USERNAME";' "$script"
  grep -Fq 'homeDirectory = "$MACHINE_HOME";' "$script"
  grep -Fq 'hostname = "$MACHINE_HOSTNAME";' "$script"
}
