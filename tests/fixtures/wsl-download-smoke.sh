#!/usr/bin/env bash
# WSL2 Ubuntu on disposable Windows GitHub-hosted runner: real transfer & executable smoke.
# Does not install Nix or modify host configuration.
set -euo pipefail

if [[ $# -ne 2 || ! $1 =~ ^[0-9]+\.[0-9]+\.[0-9]+$ || ! $2 =~ ^[0-9a-f]{64}$ ]]; then
  echo "usage: $0 VERSION PINNED_SHA256" >&2
  exit 2
fi
version=$1
sha=$2
[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || {
  echo "ERROR: expected x86_64 WSL Linux" >&2
  exit 2
}
grep -qi microsoft /proc/sys/kernel/osrelease || {
  echo "ERROR: this smoke must run inside WSL2" >&2
  exit 2
}
command -v curl
command -v sha256sum
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
url="https://github.com/NixOS/nix-installer/releases/download/$version/nix-installer-x86_64-linux"
curl --proto '=https' --tlsv1.2 --location --fail --show-error --silent \
  --retry 2 --connect-timeout 20 --max-time 150 "$url" -o "$scratch/nix-installer"
test -s "$scratch/nix-installer"
printf '%s  %s\n' "$sha" "$scratch/nix-installer" | sha256sum --check --status
chmod 0500 "$scratch/nix-installer"
"$scratch/nix-installer" --help >/dev/null
echo "PASS: WSL2 installer fetched completely, verified SHA256, and executed --help"
