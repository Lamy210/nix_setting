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
if (!v.brotli || !v.ares) {
  console.error("Brotli/c-ares runtime metadata absent; provenance unknown");
  process.exit(1);
}
'
node_path="$(node -p 'process.execPath')"
if ! command -v nix-store >/dev/null 2>&1; then
  echo 'Missing nix-store; cannot inspect dependency closure' >&2
  exit 1
fi
closure="$(nix-store --query --requisites "$node_path")"
printf '%s\n' "$closure" | grep -E '/nix/store/[^/]+-brotli-1\.2\.1(-|$)' >/dev/null || {
  echo 'Verified Brotli 1.2.1 store closure reference not found' >&2; exit 1;
}
printf '%s\n' "$closure" | grep -E '/nix/store/[^/]+-c-ares-1\.34\.9(-|$)' >/dev/null || {
  echo 'Verified c-ares 1.34.9 store closure reference not found' >&2; exit 1;
}
echo 'Runtime version and Nix store dependency names match candidate; source / ABI / binary provenance still require review.'
