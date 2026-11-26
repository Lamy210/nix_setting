# Change: Node.js 24 runtime security closure for all development shells

## Why

Node.js 24 in the SchneeForge root and standalone templates resolves independently. Current pins can select Node 24.21.0 (or older), below the October 2026 security floor. The current `nixos-unstable` package definition still reports 24.21.0 while `staging-next` contains 24.22.2 and Brotli 1.2.1. Updating a version string or only the root `flake.lock` does **not** prove that the Nix-built executable, its Brotli/c-ares closure, or an already-installed host is fixed. See #275, #265, and #271.

## What Changes

- Introduce a versioned, integrity-checked Node.js 24 security remediation policy for three independent Nix entry points: root `modules/flake-parts/devshells.nix`, `templates/node/flake.nix`, and `templates/devenv/flake.nix`.
- Select **Node.js 24.22.2 or newer in the Node 24 line**, without changing to another major, only when the package source and actual Brotli/c-ares runtime dependencies are validated. A fixed source must be immutable and reproducible.
- Prefer a released/pinned `nixos-unstable` revision that includes the complete patch closure. If unavailable, evaluate a narrowly pinned source for **Node only** (such as an exact `staging-next` commit) rather than replacing the whole package set with staging.
- Commit an auditable immutable input (`flake.lock` or equivalent exact NAR-hash pin) for the currently unlocked `templates/devenv/` template; preserve root and `templates/node/` independent lock provenance.
- Add executable and dependency-closure acceptance for Linux and Apple Silicon, including `nix develop` **execution** of the devenv template (currently skipped in CI); maintain `pnpm`/Bun interoperability and existing shell/UI/CI behavior.
- Separate remediation of repository package pins from inventory, activation, and rollback of already-running host Node binaries (including `mise`).

## Impact

- Canonical capability: `runtime-security` (new).
- Planned implementation: three Nix shell entry points, relevant input locks, deterministic CI assertions and Linux/macOS smoke; no CLI/desktop production logic changes.
- Compatibility: macOS aarch64, Linux x86_64 / aarch64 evaluation as supported; no new Intel macOS release asset.
- Release: must pass current-head required and release artifact gates before `v0.2.0-rc.8`; do not automatically release on this proposal.

## Approval and implementation gate

This PR is **design only**. It does not fix Node security. Review the source revision, ABI/closure evidence, reproducible hash, and affected inputs before approving implementation. Do not merge the later implementation PR until actual executable / library version tests and macOS acceptance pass.
