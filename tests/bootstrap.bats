#!/usr/bin/env bats

extract_detect_host() {
  sed -n '/^detect_host()/,/^}/p' "$BATS_TEST_DIRNAME/../bootstrap.sh"
}

setup() {
  detect_host_body="$(extract_detect_host)"
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
