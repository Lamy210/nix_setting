#!/usr/bin/env bash
set -eu

echo "=== Shell Startup Benchmark ==="
echo

if ! command -v hyperfine >/dev/null 2>&1; then
  echo "hyperfine not found. Run: nix develop"
  exit 1
fi

echo "zsh startup:"
hyperfine --warmup 3 --runs 10 "zsh -i -c exit"

echo
echo "zsh startup (no-rc):"
hyperfine --warmup 3 --runs 10 "zsh -f -c exit"

if command -v bash >/dev/null 2>&1; then
  echo
  echo "bash startup (for comparison):"
  hyperfine --warmup 3 --runs 10 "bash -i -c exit"
fi
