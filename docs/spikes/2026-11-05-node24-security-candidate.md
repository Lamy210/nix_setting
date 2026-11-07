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

## 2026-11-07 pinned staging bmake source availability regression

[macOS 26 CI 42249640689](https://github.com/Lamy210/nix_setting/actions/runs/42249640689) was **20/22**, with macOS 26 `nix flake check` failing to fetch `bmake-20260313.tar.gz` from `https://www.crufty.net/ftp/pub/sjg/` (HTTP 404), before the Node runtime probe could execute. The exact [pinned Nixpkgs bmake recipe](https://github.com/NixOS/nixpkgs/blob/a048ecb498d4f232d2963b19496bec9b196b990c/pkgs/by-name/bm/bmake/package.nix) declares the immutable SHA-256 `sha256-dsjzzULuBc/7R7zIElbj1edCb00I5zN4i0WYXe30+XU=`. The project's published alternative NetBSD mirror lists that same dated archive. This is an upstream source availability dependency, **not** evidence that the Node 24 binary or Brotli/c-ares is defective.

The CI macOS matrix now attempts `nix store prefetch-file` from the NetBSD mirror **only when the exact fixed-output store path is unavailable**, enforcing that original SHA-256 and expected store path. If the mirror content differs or cannot prime the same Nix store path, this step fails **closed**. This does **not** repair cold user-host builds where the upstream URL remains dead; adopt an upstream Nixpkgs mirrored-source fix or a reviewed pinned patch before release acceptance. Never bypass binary provenance/ABI or A–J host gates due solely to this CI workaround.

## 2026-11-07 native Node functional smoke (root/node/devenv, Linux/macOS)

The acceptance probe now exercises three Brotli encoder quality levels (1, 5, 11) with round-trip decoding and exact byte comparison, a `dns.lookup("localhost")` OS-resolver callback (no external DNS or network access), and OpenSSL TLS secure-context creation. Any native operation failure exits nonzero, independent of the exact `process.versions` and dynamic-link assertions. This verifies **basic runtime functionality** across the two intentionally mixed Nixpkgs package closures. It does **not** establish full native addon ABI compatibility, ELF `PT_INTERP`, unsupported third-party modules, static/bundled code provenance, actual user host installation, or signed/released RC.8 assets.

## 2026-11-07 c-ares functional resolver smoke (offline)

`dns.lookup("localhost")` only exercises OS `getaddrinfo`, **not** c-ares. Added a local UDP fixture that returns a deterministic single A record for `schneeforge-probe.invalid`, and a new Node `dns.Resolver` configured to `127.0.0.1:<ephemeral-port>` calls `resolve4`. This executes the actual c-ares DNS query/parse path without relying on internet DNS, public resolvers, or mutable external data. The test closes the UDP socket and fails closed after five seconds. It runs alongside Brotli encode/decode at qualities 1/5/11, TLS context creation, strict shared-library paths, process.versions and Nix closure validation on each supported Node shell.

The fixture was executed successfully using a separate local Node 22 runtime as a **PoC only**; final security conclusions require the pinned Node 24.22.2 CI matrix. This still does **not** demonstrate security across unrelated host Node processes, standalone native add-ons or RC.8 distribution assets.

## 2026-11-07: remove cache seeding after checksum-preserving bmake source policy

The [repository-level mirror policy #303](https://github.com/Lamy210/nix_setting/pull/303) merged as `6e3d7e94bcf885502032092d15a77e04e8e2833a`, after 22/22 required CI. This candidate now imports both the main `nixpkgs` and the pinned `node-security-nixpkgs` with the exact `config.rewriteURL` for the bmake 20260313 archive in root and copied standalone templates.

The previous macOS CI `Seed verified bmake mirror` fixed-output cache priming is removed. It may no longer mask the bad upstream URL on a cold runner. The original Nixpkgs fixed-output SHA-256 remains mandatory. Re-run macOS 15/26 and Linux from the **new** candidate HEAD; prior 22/22 results in [run 42277850251](https://github.com/Lamy210/nix_setting/actions/runs/42277850251) are historical and cannot substitute for current-head checks.

This is an environment and package-source fix, not proof of unrelated user-host Node/mise processes, third-party native add-on ABI, signed assets or GUI installer A–J. Keep RC.8 untagged until the release and host gates are separately met.

## 2026-11-07 mixed-closure ABI execution gates

Latest previous-head CI [42342923972](https://github.com/Lamy210/nix_setting/actions/runs/42342923972) was **22/22** including macOS 15/26 Node executable checks *without* the CI-only bmake cache seed. To disambiguate the Linux `ldd` report which mapped a glibc 2.44 `ld-linux` path to an older glibc 2.42 runtime, the probe now parses the Node ELF binary's **actual `PT_INTERP`** using `readelf`, rejects non-Nix or absent interpreters, checks the interpreter and directly-linked `libc.so.6` belong to the *same* store output, and requires that output in the Node closure. This verifies actual interpreter provenance rather than treating `ldd`'s process mapping as conclusive.

Each of the three independent Linux/macOS Node shells also compiles and `require()`-loads a small Node-API C addon, testing the stable `napi_register_module_v1` entrypoint, `napi_create_int32`, and `napi_set_named_property`. It uses local shell-provided `cc`, an ephemeral directory, and no npm install, third-party code, or internet access. An addon that does not compile/load or return the expected value fails the probe. The new checks are **not** proof of third-party native C++ addons, arbitrary libc++ ABI compatibility, production host Node installation, or Apple Silicon physical A–J. Require a new latest-head full CI before claiming successful execution.

## 2026-11-07: pnpm package-manager runtime security evidence and repair

### Confirmed vulnerability in independent Node template

The actual Linux CI run [42359540999](https://github.com/Lamy210/nix_setting/actions/runs/42359540999) found that the standalone Node template exposed **Node 24.22.2** on `PATH`, yet its `pnpm 11.20.0` launcher executed with **Node 24.18.1**. The original Node/Brotli/c-ares and compiled Node-API probes succeeded, which proves that checking only the visible `node` misses this separate runtime. This is a confirmed regression, not a speculation.

### Why the root and template require different implementations

We inspected the **exact immutable Nixpkgs package sources** from both locks:

- Root lock `fe13242281a67641c0102cdb1087f0ebe586c7df` packages default **pnpm 12** as a compiled Rust executable in `pkgs/development/tools/pnpm/generic-rust.nix`. It does not accept a `nodejs` override and does not have an independent Node interpreter.
- Standalone Node template lock `2fcb964de67fcf60b43471c55d5d99e61a9ccb5a` packages **pnpm 11** as a Node-based launcher in `pkgs/development/tools/pnpm/generic.nix`, whose accepted interpreter override is **`nodejs-slim`**.

An initial attempt to apply `pkgs.pnpm.override { nodejs = nodePatched; }` uniformly caused the actual Nix `flake-check` to fail because the Rust derivation has no `nodejs` function argument. That attempt was **rejected**; it is not the current implementation.

### Implemented correction and acceptance policy

- Root retains the native pnpm 12 executable. The offline CI smoke requires a Nix-store pathname, confirms ELF/Mach-O native-binary format and reviewed pnpm major 12, and executes a local project script using the patched Node.
- Standalone Node template binds `pkgs.pnpm.override { nodejs-slim = nodePatched; }`. The smoke uses a controlled `NODE_OPTIONS=--require` preload to inspect **pnpm's own Node process**, rejecting versions below Node 24.22.2 or missing Brotli 1.2.1/c-ares 1.34.9. A missing instrumented process is not counted as success.
- Both paths run a local `pnpm run` script and a `bun run` smoke for the pinned Bun 1.4.4; they never install packages or call a registry. The separate devenv template does not promise pnpm/Bun, but retains its mandatory Node runtime tests.
- Root and standalone Node shell checks run in Linux plus macOS 15/26. This is an execution-level regression gate; additional package sources and physical user-host installations are outside its scope.

The corrected standalone template ran its security toolchain smoke successfully on Linux in the current follow-up CI (commit `b577315`); require completed current-head root/macos checks as independent evidence before accepting the candidate. Third-party C++ addons, host/mise inventory, supply-chain distribution and provisional RC.8 remain separate acceptance gates.
