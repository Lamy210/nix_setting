#!/usr/bin/env bash
set -euo pipefail

# Offline acceptance smoke for the Node 24 devShell and standalone node template.
# In particular, pnpm's own interpreter must be patched, not merely the
# PATH-selected node used by the package's child scripts.
for tool in node pnpm bun file; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Missing required Node development tool: $tool" >&2
    exit 1
  fi
done

probe_dir="$(mktemp -d)"
trap 'rm -rf "$probe_dir"' EXIT
cat >"$probe_dir/verify-node.cjs" <<'NODE'
'use strict';
const assert = require('node:assert/strict');
const { versions, execPath } = require('node:process');
const [major, minor, patch] = versions.node.split('.').map(Number);
assert.equal(major, 24, 'unexpected Node major');
assert.ok(minor > 22 || (minor === 22 && patch >= 2), 'outdated Node 24 binary');
assert.equal(versions.brotli, '1.2.1', 'outdated Brotli runtime');
assert.equal(versions.ares, '1.34.9', 'outdated c-ares runtime');
assert.ok(execPath.startsWith('/nix/store/'), 'Node runtime outside Nix store');
console.log('Patched Node 24 runtime under Nix store: OK');
NODE
cat >"$probe_dir/preload.cjs" <<'NODE'
'use strict';
// NODE_OPTIONS runs inside pnpm's own Node interpreter, before pnpm starts.
// A pnpm wrapper that never executes Node cannot prove this invariant.
require('./verify-node.cjs');
require('node:fs').appendFileSync(
  process.env.SCHNEEFORGE_PNPM_NODE_AUDIT,
  process.execPath + '\n',
);
NODE
cat >"$probe_dir/verify-bun.mjs" <<'BUN'
import assert from 'node:assert/strict';
assert.equal(Bun.version, '1.4.4', 'unexpected Bun security backport version');
console.log('Bun 1.4.4 runtime: OK');
BUN
cat >"$probe_dir/package.json" <<'PACKAGE'
{
  "name": "schneeforge-security-toolchain-smoke",
  "version": "0.0.0",
  "private": true,
  "scripts": {
    "probe:node": "node ./verify-node.cjs",
    "probe:bun": "bun ./verify-bun.mjs"
  }
}
PACKAGE

(
  cd "$probe_dir"
  # No installation or registry access: check only local runtimes and CLI execution.
  node ./verify-node.cjs
  pnpm_path="$(command -v pnpm)"
  if [[ $pnpm_path != /nix/store/* ]]; then
    echo 'pnpm does not originate from the Nix store' >&2
    exit 1
  fi
  # Independently pinned nixpkgs revisions package pnpm differently:
  # root pnpm 12 is a compiled Rust program; standalone pnpm 11 is Node JS.
  if file -Lb "$pnpm_path" | grep -Eq 'ELF|Mach-O'; then
    pnpm_version="$(pnpm --version)"
    if [[ $pnpm_version != 12.* ]]; then
      echo "Unreviewed native pnpm version: $pnpm_version" >&2
      exit 1
    fi
    echo "Native Rust pnpm $pnpm_version: no Node interpreter"
    pnpm run probe:node
  else
    # Fail before executing a vulnerable pnpm 11 JS runtime. The preload
    # executes in pnpm's process, not merely in a child project script.
    SCHNEEFORGE_PNPM_NODE_AUDIT="$probe_dir/pnpm-runtime.log" \
      NODE_OPTIONS="--require=$probe_dir/preload.cjs" pnpm --version
    if [ ! -s "$probe_dir/pnpm-runtime.log" ]; then
      echo 'pnpm did not expose its Node process for runtime verification' >&2
      exit 1
    fi
    SCHNEEFORGE_PNPM_NODE_AUDIT="$probe_dir/pnpm-runtime.log" \
      NODE_OPTIONS="--require=$probe_dir/preload.cjs" pnpm run probe:node
  fi
  bun run probe:bun
)
echo 'Node + pnpm + Bun offline toolchain smoke: OK'
