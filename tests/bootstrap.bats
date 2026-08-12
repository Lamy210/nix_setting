#!/usr/bin/env bats

extract_detect_host() {
  sed -n '/^detect_host()/,/^}/p' "$BATS_TEST_DIRNAME/../bootstrap.sh"
}

setup() {
  detect_host_body="$(extract_detect_host)"
}

@test "detect_host returns macbook-air on macOS arm64" {
  uname() {
    case "$1" in
      -s) echo "Darwin" ;;
      -m) echo "arm64" ;;
    esac
  }
  eval "$detect_host_body"
  result="$(detect_host)"
  [ "$result" = "macbook-air" ]
}

@test "detect_host returns macbook-air on macOS x86_64" {
  uname() {
    case "$1" in
      -s) echo "Darwin" ;;
      -m) echo "x86_64" ;;
    esac
  }
  eval "$detect_host_body"
  result="$(detect_host)"
  [ "$result" = "macbook-air" ]
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

@test "detect_host returns unknown on unsupported OS" {
  uname() {
    case "$1" in
      -s) echo "FreeBSD" ;;
      -m) echo "x86_64" ;;
    esac
  }
  eval "$detect_host_body"
  result="$(detect_host)"
  [ "$result" = "unknown" ]
}
