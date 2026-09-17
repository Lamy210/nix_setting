#!/usr/bin/env bash
set -euo pipefail

version_file=/tmp/schneeforge-canary-version

if [[ ! -r $version_file ]]; then
  printf 'missing canary version file: %s\n' "$version_file" >&2
  exit 90
fi

app_version="$(tr -d '\r\n' <"$version_file")"

case "${1:-}" in
__backend-info)
  printf '{"protocol_version":1,"app_version":"%s","os":"linux","arch":"%s"}\n' \
    "$app_version" "$(uname -m)"
  ;;
canary-transport)
  if [[ ${2:-} != 'a b;$(x)' ]]; then
    printf 'unexpected argv payload: %q\n' "${2:-}" >&2
    exit 91
  fi
  printf 'canary-argv=%s\n' "$2"
  exit 23
  ;;
*)
  printf 'unexpected canary command: %q\n' "${1:-}" >&2
  exit 92
  ;;
esac
