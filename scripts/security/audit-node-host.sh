#!/usr/bin/env bash
set -euo pipefail

# Read-only evidence collection for Node 24 host runtime security (issue #275).
# No installations, process termination, network calls or privilege escalation.
if [ "$#" -ne 0 ]; then
  printf 'Usage: %s (no arguments)\n' "$0" >&2
  exit 2
fi

printf '%s\n' 'Node host runtime audit — local evidence only'
printf 'OS=%s architecture=%s\n' "$(uname -s)" "$(uname -m)"
if ! command -v node >/dev/null 2>&1; then
  echo 'Node not found on PATH. Status: UNKNOWN (exit 2)' >&2
  exit 2
fi
printf '%s\n' 'PATH-visible Node candidates:'
type -a node || true

# Avoid user-supplied NODE_OPTIONS/NODE_PATH from injecting requires into the
# inspected runtime. This does not change the caller's environment.
runtime_status=0
(
  unset NODE_OPTIONS NODE_PATH
  node <<'NODE'
const v = process.versions;
const numbers = (value) => String(value || "").split(".").map((piece) => Number(piece));
const atLeast = (actual, floor) => {
  const a = numbers(actual);
  const b = numbers(floor);
  for (let i = 0; i < b.length; i++) {
    if ((a[i] || 0) > b[i]) return true;
    if ((a[i] || 0) < b[i]) return false;
  }
  return true;
};
const nodeNumbers = numbers(v.node);
const nodeMajor = nodeNumbers[0];
const node24Floor = nodeMajor === 24 && atLeast(v.node, "24.22.2");
const depFloor = atLeast(v.brotli, "1.2.1") && atLeast(v.ares, "1.34.9");
const status = nodeMajor === 24 ? (node24Floor && depFloor ? "PASS" : "FAIL") : "REVIEW";
console.log(JSON.stringify({
  status,
  platform: process.platform,
  arch: process.arch,
  execPath: process.execPath,
  node: v.node,
  brotli: v.brotli || null,
  ares: v.ares || null,
  openssl: v.openssl || null
}, null, 2));
if (status === "FAIL") console.error("Active Node 24 is below the audited Node/Brotli/c-ares floor");
if (status === "REVIEW") console.error("Different Node major: its own security support window must be assessed separately");
process.exitCode = status === "PASS" ? 0 : status === "FAIL" ? 1 : 2;
NODE
) || runtime_status=$?

node_path="$( (unset NODE_OPTIONS NODE_PATH; node -p 'process.execPath') )"
if [[ "$node_path" == /nix/store/* ]]; then
  echo 'Active executable is under /nix/store; checking local closure (read-only).'
  if command -v nix-store >/dev/null 2>&1; then
    if ! nix-store --query --requisites "$node_path" | grep -E -- '(-brotli-|-c-ares-|-glibc-)'; then
      echo 'Could not inspect matching Nix dependencies; manual review required.' >&2
    fi
  else
    echo 'nix-store is unavailable; closure not verified.' >&2
  fi
  if [ "$(uname -s)" = Darwin ] && command -v otool >/dev/null 2>&1; then
    echo 'Node Mach-O dependencies relevant to TLS/ABI:'
    otool -L "$node_path" | grep -E -- '(brotli|cares|libc\+\+|libSystem)' || true
  elif [ "$(uname -s)" = Linux ] && command -v readelf >/dev/null 2>&1; then
    echo 'Node ELF interpreter (not a claim of native-addon ABI):'
    readelf -l "$node_path" | grep 'Requesting program interpreter:' || true
  fi
else
  echo 'Active Node is outside /nix/store. Updating this repository Nix shell does not update that binary.'
fi

if command -v mise >/dev/null 2>&1; then
  echo 'mise is installed; inspect all managed Node versions separately with: mise ls --installed node'
fi

echo 'This audits the PATH-selected Node only; other installed versions and running processes remain unverified.'
echo 'Redact user names and private filesystem paths before sharing the report.'
exit "$runtime_status"
