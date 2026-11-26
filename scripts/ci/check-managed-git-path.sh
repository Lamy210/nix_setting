#!/usr/bin/env bash
# Run inside the actual maintenance devShell; never accept the runner's Git.
set -euo pipefail
expected="${1:?expected managed Git output path required}/bin/git"
actual="$(command -v git)"
printf 'Git on PATH: %s\nExpected Git: %s\n' "$actual" "$expected"
test "$actual" -ef "$expected"
git --version
