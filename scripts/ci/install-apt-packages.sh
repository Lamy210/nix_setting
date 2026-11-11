#!/usr/bin/env bash
# Install mandatory CI dependencies without treating transient APT DNS outages
# as a product failure. Exhausted retries are fatal: never skip an E2E gate.
set -euo pipefail

if (($# == 0)); then
  echo "Usage: install-apt-packages.sh <package> [package ...]" >&2
  exit 2
fi

# Keep individual attempts bounded. APT sometimes stalls while resolving
# azure.archive.ubuntu.com on GitHub-hosted ubuntu-24.04 runners.
for attempt in 1 2 3; do
  printf 'APT dependencies (attempt %d/3): %s\n' "$attempt" "$*"
  if timeout --signal=TERM --kill-after=5s 120s \
    sudo apt-get -o Acquire::Retries=1 \
    -o Acquire::http::Timeout=15 -o Acquire::https::Timeout=15 \
    -o APT::Update::Error-Mode=any update -qq &&
    timeout --signal=TERM --kill-after=5s 180s \
      sudo apt-get -o Acquire::Retries=1 \
      -o Acquire::http::Timeout=15 -o Acquire::https::Timeout=15 \
      install -y -qq -- "$@"; then
    echo "PASS: mandatory APT dependencies installed: $*"
    exit 0
  fi
  if ((attempt == 3)); then
    echo "ERROR: mandatory APT dependencies unavailable after 3 bounded attempts: $*" >&2
    exit 1
  fi
  echo "APT attempt $attempt failed; retrying after backoff (no CI gate bypass)" >&2
  sleep $((attempt * 10))
done
