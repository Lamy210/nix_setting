# Node host runtime security audit (read-only)

Issue [#275](https://github.com/Lamy210/nix_setting/issues/275) tracks the distinction between **Nix devShell Node** and the **actual PATH-selected Node**, optional `mise` runtimes, and any already-running processes. CI [#299](https://github.com/Lamy210/nix_setting/pull/299) cannot inspect the user's physical Mac or safely change its globally installed Node.

## Operator procedure

From the repository checkout, run `bash scripts/security/audit-node-host.sh` **on an authorized disposable test Mac first**. It executes the PATH-selected Node with `NODE_OPTIONS` and `NODE_PATH` unset in a subshell, prints its actual executable path and runtime dependency versions, and queries local Nix closure/Mach-O or ELF interpreter metadata where applicable. No downloaded tools, Nix builds, installs, process termination, privilege escalation, network requests, Gatekeeper overrides or configuration edits are performed.

Exit-code interpretation: **0** means the selected Node **24** meets Node >=24.22.2, Brotli >=1.2.1 and c-ares >=1.34.9 *version floors only*; **1** means selected Node 24 fails one of those floors; **2** means no Node is on PATH or a different Node major needs a separate support/advisory assessment. A successful exit is **not** an assertion that there are no alternative or currently running vulnerable Node instances, that static/bundled libraries are safe, or that the OS has accepted the macOS distribution.

For complete acceptance, an authorized operator must independently enumerate `mise ls --installed node`, other version managers, Homebrew/system executables and active Node processes, and record activation/rollback. Do **not** post unredacted home-directory paths, process arguments, tokens or other private data to public GitHub issues. In particular, do not use broad process command-line capture in an issue.

Use this audit alongside `docs/testing/macOS-final-acceptance-checklist.md`, not as a replacement for official RC build artifacts, fresh-install, Gatekeeper/Finder and A–J acceptance.
