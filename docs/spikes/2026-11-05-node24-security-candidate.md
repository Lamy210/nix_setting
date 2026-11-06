# Node 24.22.2 narrowly pinned staging candidate — unaccepted

This **Draft only** uses exact NixOS/nixpkgs commit `a048ecb498d4f232d2963b19496bec9b196b990c` to supply **only** `nodejs_24` to three separately resolved development-shell entrypoints. The regular `nixpkgs` input used by other packages remains unchanged. On inspected upstream sources at this commit, Node is 24.22.2; Brotli 1.2.1; c-ares 1.34.9. This is source evidence, **not** proof of actual compiled runtime or vulnerability remediation.

## Evidence still required before Ready for review

1. Use Nix to generate and **commit** correct new `flake.lock` entries in root and Node template, and a full standalone `templates/devenv/flake.lock`. Preserve independent input provenance; verify locked `narHash` against upstream.
2. Run `nix develop .#node --command bash scripts/ci/check-node24-candidate.sh`, and execute the same probe within separately copied Node and devenv templates. Use an absolute path to the probe outside the copy.
3. Capture `node -p 'JSON.stringify(process.versions)'`, `nix-store -qR` closure paths, `ldd`/ `otool -L` links, upstream source/patchset and transitive library ABI for macOS 15/26 and Linux x86_64; record Linux aarch64 eval separately. Static/bundled linkage requires independent source evidence.
4. Review mixed Nixpkgs closure size and dynamic-library compatibility vs narrow overlay. Test Node+pnpm+Bun and devenv services. The script's dependency-name assertions are only a first **fail-closed** probe, not complete provenance.
5. Enable actual executable CI gates for all three shells (current devenv template skips devShell execution), add negative tests, then pass all required checks. Reconcile Draft #257 so it cannot downgrade the Node package.
6. Track actual host-installed Node versions / mise separately. No host activation, updater enabling, `v0.2.0-rc.8` tag, release asset, or false security-closure claim is allowed from this Draft.

Fallback: if released `nixos-unstable` later includes all patches and passes real runtime inspection, replace the extra staging input with reviewed stable locks.

## Devenv flake evaluation

The standalone devenv template uses `devenv.lib.mkShell`. Its standard `nix develop` pure evaluation cannot resolve the working directory; the official devenv flake guide requires `nix develop --no-pure-eval`. The CI template runtime probe explicitly uses that flag (only for the devenv template) rather than setting an absolute `devenv.root`, which would make the template non-portable. Source: https://devenv.sh/guides/using-with-flakes/ .

## Cross-platform runtime gate

The macOS 15/26 matrix now executes the actual Node runtime and `nix-store --query --requisites` candidate check in **root**, independent **Node template**, and independent **devenv template** development shells before Home Manager / nix-darwin builds. This is required for security P0 review; a failing or unsupported shell will fail the matrix, never be silently skipped. The full CI and specific executable provenance must still be reviewed after completion.

## 2026-11-06 observed macOS CI evidence and remaining proof boundaries

- Parent Draft head `5aa8af14349ea91b3a43c29ac56779cf60c85d38` at [Actions run 42142118855](https://github.com/Lamy210/nix_setting/actions/runs/42142118855): the macOS 15/Xcode 26.3 and macOS 26/Xcode 26.6 **Node 24 security candidate runtime and Nix closure** steps both completed successfully. Each CI step invokes the root, standalone Node and standalone devenv shells sequentially with `set -euo pipefail`; the Linux counterparts likewise passed in [42133132799](https://github.com/Lamy210/nix_setting/actions/runs/42133132799). The parent run's OpenSpec + aggregate required CI failed only because it predated separately merged #300. This prior success is a regression baseline, **not** a substitute for rerunning latest merged branch.
- The probe now rejects any Node with runtime Brotli other than **1.2.1** or c-ares other than **1.34.9**, and any executable not in `/nix/store`. It collects `otool -L` (Mach-O) or `ldd` (ELF) output and an executable SHA-256 fingerprint, and requires Nix closure entries for both libraries. Dynamic linkage output alone cannot prove static or bundled code was patched; runtime metadata + Nix recipe/closure review remain necessary.
- Distribution checks are separate from Node shell checks. [release-artifact-check](https://github.com/Lamy210/nix_setting/actions/workflows/check.yml) tests unsigned macOS CLI/DMG build smoke + portability and Linux musl build paths, but cannot attest the official **tag-built** assets, CHECKSUMS, SBOM/provenance, notarization state or downloaded-asset behavior before publishing a release.
- **Actual user-host proof is outstanding**: this repository's Nix devShell pin does not update host `node`, `mise` runtimes or running processes. An authorized disposable Apple Silicon host must record OS/architecture, `command -v node`, `node -p 'JSON.stringify(process.versions)'`, `node -p 'process.execPath'`, `otool -L`, `nix-store -qR`, installation/rollback results and Final Acceptance A–J. Do not alter the user's real host or disable Gatekeeper as a workaround.
- Remain Draft until latest-head CI is green, source/build provenance and mixed-closure ABI implications are reviewed, and actual host acceptance/supply-chain release gates are passed. No RC.8 publication authorized by these checks.

## 2026-11-07 direct dynamic-link rejection gate

Prior successful [CI 42229539118](https://github.com/Lamy210/nix_setting/actions/runs/42229539118) recorded `otool -L` links for all three independently locked Node shells on both macOS 15/26, resolving both Brotli decoder/encoder dylibs to **1.2.1** and c-ares dylib to **1.34.9**. Linux Node template `ldd` resolved both Brotli .so libraries and c-ares .so to those same fixed Nix Store packages. The updated probe now **fails closed** if the direct dynamic-link output doesn't show both patched Brotli libraries and patched c-ares, in addition to process.versions checks and Nix requisites. Bundled/static libraries, host Node installation and mixed libc ABI remain separate unverified surfaces.

Linux `ldd` logged references to glibc **2.44** and a loader line mapped to glibc **2.42**. Since that line may reflect `ldd` itself, it is **not sufficient evidence** of an actual interpreter mismatch; next ABI checks should capture ELF `PT_INTERP` and native-module load behavior. macOS Nix Node uses its own libcxx 20 dynamic dependency even where unrelated devShell packages use libcxx 21; confirm cross-package native-addon ABI before accepting the security candidate into a release.
