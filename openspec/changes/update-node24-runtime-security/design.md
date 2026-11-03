# Design: Node.js 24 security-only remediation

## Context and evidence

- Three independent Nix inputs resolve `pkgs.nodejs_24`: root devShell, Node template, and devenv template.
- Current `nixos-unstable` `v24.nix` was observed at `24.21.0` and Brotli package at `1.2.0` on 2026-11-04. This is source evidence, not a measured runtime closure.
- Upstream `staging-next` at `a048ecb498d4f232d2963b19496bec9b196b990c` was observed in #275 with Node 24.22.2 / Brotli 1.2.1, but c-ares ABI/version, actual built runtime, and host state remain **unknown**.

## Goals / Non-goals

- Goals: proven patched runtime with immutable source, independent inputs covered, deterministic CI failures on regressions, minimal unrelated dependency drift.
- Non-goals: installing or upgrading Node on existing user hosts, changing Node major, enabling automatic host upgrades, replacing the entire root Nix closure with staging, bypassing insecure dependency checks.

## Alternatives and decision framework

| Option | Benefits | Risks / acceptance |
|---|---|---|
| A. Promote released nixos-unstable revision separately in root/templates | Native Nix packaging, smallest maintenance burden | Wait for all security patches; review unrelated lock changes and activation, verify each independent lock |
| B. Pinned staging-next Node-only input | Can isolate urgency without upgrading all packages | Two nixpkgs closures, closure/ABI duplication, longer builds; exact commit/hash and runtime check are mandatory |
| C. Targeted source/derivation overlay on existing nixpkgs | Potentially smaller store changes | Node's source, Brotli and c-ares linkage are complex; local patch maintenance and missed transitive fixes |
| D. Official prebuilt Node tarballs | Potentially quick | Linux ELF loader/ABI and Darwin packaging, provenance/architecture, Nix purity and transitive library proof harder |

**Provisional preference:** A if a reviewed released revision supplies all fixes; otherwise compare B and C with reproducible build evidence. Do not select on version number alone. Avoid broad staging input replacement.

## Required evidence before implementation is accepted

1. Record exact source SHA, Nix NAR hash and lock of all three flake roots, and verify upstream identity/checksums through authenticated sources.
2. Build/evaluate `nix develop .#node`, `nix develop ./templates/node`, and `nix develop ./templates/devenv`; execute the resulting `node` and assert `process.version` Node 24 >=24.22.2.
3. Capture the resolved `process.versions.brotli` and `process.versions.ares`, Nix store closure, ELF or Mach-O dependency linking, and compare against independently verified patched Brotli/c-ares versions / patchset. Version-only assertions SHALL NOT be sufficient.
4. Test macOS 15/26, Linux x86_64, Linux aarch64 evaluation and supported `pnpm`/Bun workflows without weakening required checks. Explicitly record skipped execution targets.
5. Inventory actual host-installed and actively running Node instances separately; require opt-in activation, a version after-switch check and rollback evidence before claiming an individual host remediated.
6. Reconcile any future #257 root `flake.lock` merge so it cannot silently restore an older Node package.

## Risks and rollback

- Mixing Nixpkgs revisions can increase store size, build time or cause ABI mismatch: measure derivation closure size, runtime `ldd` / `otool -L`, build and launch.
- If dependency attribution is ambiguous, mark **unknown** and keep the P0 release gate closed.
- Rollback a failing candidate by reverting its lock and overlay/input commit; preserve previous user-host runtime until a safe migration is explicitly accepted.
