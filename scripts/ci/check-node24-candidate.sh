#!/usr/bin/env bash
set -euo pipefail

# Experimental acceptance probe for the Node 24 security candidate.
# A version-only check is never sufficient evidence for a patched closure.
node -e '
const v = process.versions;
const [major, minor, patch] = v.node.split(".").map(Number);
if (major !== 24 || minor < 22 || (minor === 22 && patch < 2)) {
  console.error("Node 24 >=24.22.2 required, got " + v.node);
  process.exit(1);
}
console.log(JSON.stringify({node:v.node, brotli:v.brotli, ares:v.ares, openssl:v.openssl, execPath:process.execPath}));
if (v.brotli !== "1.2.1" || v.ares !== "1.34.9") {
  console.error("Patched Brotli 1.2.1 / c-ares 1.34.9 required, got " + v.brotli + " / " + v.ares);
  process.exit(1);
}
'
node_path="$(node -p 'process.execPath')"
if ! command -v nix-store >/dev/null 2>&1; then
  echo 'Missing nix-store; cannot inspect dependency closure' >&2
  exit 1
fi
case "$node_path" in
  /nix/store/*) ;;
  *) echo 'Node binary does not originate from the Nix store; refusing unverifiable host binary' >&2; exit 1 ;;
esac
echo "Node executable: $node_path"
case "$(uname -s)" in
  Darwin)
    if ! command -v otool >/dev/null 2>&1; then
      echo 'otool missing: cannot record Mach-O dependency linkage' >&2
      exit 1
    fi
    echo 'Mach-O linked dynamic libraries:'
    otool -L "$node_path"
    ;;
  Linux)
    if ! command -v ldd >/dev/null 2>&1; then
      echo 'ldd missing: cannot record ELF dependency linkage' >&2
      exit 1
    fi
    echo 'ELF linked dynamic libraries:'
    ldd "$node_path"
    ;;
  *)
    echo "Unsupported host OS: $(uname -s)" >&2
    exit 1
    ;;
esac
if command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "$node_path"
elif command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$node_path"
else
  echo 'No SHA-256 digest tool available for executable fingerprint' >&2
  exit 1
fi
closure="$(nix-store --query --requisites "$node_path")"
printf '%s\n' "$closure" | grep -E '/nix/store/[^/]+-brotli-1\.2\.1(-|$)' >/dev/null || {
  echo 'Verified Brotli 1.2.1 store closure reference not found' >&2
  exit 1
}
printf '%s\n' "$closure" | grep -E '/nix/store/[^/]+-c-ares-1\.34\.9(-|$)' >/dev/null || {
  echo 'Verified c-ares 1.34.9 store closure reference not found' >&2
  exit 1
}
echo 'Runtime version and Nix store dependency names match candidate; source / ABI / binary provenance still require review.'
