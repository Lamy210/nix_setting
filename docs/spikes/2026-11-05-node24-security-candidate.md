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
