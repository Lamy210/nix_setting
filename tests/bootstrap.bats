#!/usr/bin/env bats

extract_detect_host() {
  sed -n '/^detect_host()/,/^}/p' "$BATS_TEST_DIRNAME/../bootstrap.sh"
}

extract_flake_functions() {
  sed -n '/^nix_has_required_flake_features()/,/^}/p' "$BATS_TEST_DIRNAME/../bootstrap.sh"
  sed -n '/^ensure_flakes_enabled()/,/^}/p' "$BATS_TEST_DIRNAME/../bootstrap.sh"
}

setup() {
  detect_host_body="$(extract_detect_host)"
  flake_functions="$(extract_flake_functions)"
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
