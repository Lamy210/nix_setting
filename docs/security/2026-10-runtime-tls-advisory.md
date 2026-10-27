# October 2026 runtime and TLS trust advisory — SchneeForge

**Reviewed: 2026-10-27** · Operational incident guidance, not a proof of deployed-host remediation. This guide is not a normative product specification; future policy changes require an approved OpenSpec change.

## Scope and evidence: separate source pins, evaluated packages, deployed binaries

SchneeForge uses a pinned `nixpkgs` flake input, Python/Node development shells and templates, and an **opt-in** Claude Code Home Manager module. The repository has **no PHP package or PHP serving configuration found in this audit**; actual user machines, containers, and external apps were not inventoried. Absence from this repository does not prove that a machine or a different service has no PHP.

For the exact `nixpkgs` input locked in `develop` (`fe13242281a67641c0102cdb1087f0ebe586c7df`), package *definitions* were read at their pinned commit; these are **not measurements of installed host binaries**. Draft PR #257 (`3ec636362e1816bc2f0888fa0fa2c69f1bc9e504`) is the existing candidate. Neither commit was modified by this advisory.

| Component | Current locked definition | PR #257 candidate definition | Required response |
| --- | --- | --- | --- |
| `uv` | 0.13.0 | 0.13.4 | Current definition is below the eMudhra mitigation floor **0.13.4**. Candidate meets the uv floor but only fixes installations that actually activate it. Update all relevant developer shells, cached OCI images, uv-managed Python and venvs independently. |
| `bun` | 1.4.2 | **1.4.2** | **Both are below Bun 1.4.4**, which patches bundled Brotli CVE-2026-111497. Do NOT label PR #257 a complete security fix. Prepare a separate, verified nixpkgs update/override to 1.4.4 or later, covering all advertised platforms. |
| `claude-code` | 2.1.300 | 2.1.308 | Both exceed the 2.1.251 floor for GHSA-5xjx-9qx6-jcrv; official v2.1.311 was released 2026-10-26. The module `modules/experimental/ai.nix` is commented out in `modules/default.nix`. An enabled environment must validate **its executable** and consider updating to 2.1.311 or newer, after release review. |
| `nodejs_24` | Version **not established** by this audit | Version **not established** | Verify selected runtime is >=24.22.1 for the affected Node 24 line. Node 22/26 floors are 22.23.4/26.12.1. Do not assume a flake revision fixes Node unless the exact package and runtime match. |
| `python3`, `pip`, `certifi` | Versions **not established** | Versions **not established** | Verify interpreter and installed/bundled CA stores individually, including `ensurepip` and existing venvs. Recommended patched Python lines: 3.11.18, 3.12.16, 3.13.18, 3.14.10, 3.15.2 (or newer supported patches); pip >=26.2.2, certifi >=2026.10.19. |
| PHP | **No PHP runtime/deployment declaration found** | Not affected by PR #257's lockfile intent | Scan external hosts/containers separately. Apply patched PHP 8.2.35, 8.3.36, 8.4.27 or 8.5.12 (or later). Replace EOL branches; additional CVE-2026-115560 has a narrow, separately disclosed condition. |

### Primary sources and exact pinned definitions

- PHP vendor: https://www.php.net/archive/2026.php#2026-10-21-5 and https://www.php.net/ ; IPA https://www.ipa.go.jp/security/security-alert/2026/alert20261022.html ; CERT-EU https://www.cert.europa.eu/publications/security-advisories/2026-018
- Bun vendor: https://github.com/oven-sh/bun/security/advisories/GHSA-663w-cv8g-jj2p (>=1.4.4)
- Node vendor: https://nodejs.org/en/blog/vulnerability/october-2026-security-releases ; patched 24.22.1 / 22.23.4 / 26.12.1.
- Anthropic: https://github.com/anthropics/claude-code/security/advisories/GHSA-5xjx-9qx6-jcrv (patched 2.1.251), https://github.com/anthropics/claude-code/releases/tag/v2.1.311
- eMudhra: https://emudhra.com/en/post/certificate-security-incident/ ; Python security notice https://discuss.python.org/t/information-for-python-users-regarding-emudhra-emsign-ca-certificate-security-incident/109576 ; certifi https://github.com/advisories/GHSA-6q48-6c7w-9gw2 ; uv https://github.com/astral-sh/uv/security/advisories/GHSA-hq85-5cpj-mr62
- Current nixpkgs evidence: https://github.com/NixOS/nixpkgs/blob/fe13242281a67641c0102cdb1087f0ebe586c7df/pkgs/by-name/uv/uv/package.nix ; https://github.com/NixOS/nixpkgs/blob/fe13242281a67641c0102cdb1087f0ebe586c7df/pkgs/by-name/bu/bun/package.nix ; https://github.com/NixOS/nixpkgs/blob/fe13242281a67641c0102cdb1087f0ebe586c7df/pkgs/by-name/cl/claude-code/manifest.zst.json
- PR #257 input definitions: https://github.com/NixOS/nixpkgs/blob/3ec636362e1816bc2f0888fa0fa2c69f1bc9e504/pkgs/by-name/uv/uv/package.nix ; https://github.com/NixOS/nixpkgs/blob/3ec636362e1816bc2f0888fa0fa2c69f1bc9e504/pkgs/by-name/bu/bun/package.nix ; https://github.com/NixOS/nixpkgs/blob/3ec636362e1816bc2f0888fa0fa2c69f1bc9e504/pkgs/by-name/cl/claude-code/manifest.zst.json

## Threats and priority

**P0 — inspect and patch if deployed:** PHP2Shell (CVE-2026-109618 / -109619) may permit unauthenticated RCE in PHP request handlers; only PHP-serving assets are directly exposed. Public PHP 8.2–8.5 fixed floors above. The October 23 informational CVE-2026-115560 is **not fixed by asserting these floors alone**: vendor mitigation is `session.upload_progress.enabled=0` pending a later patch, but only a rare runtime configuration/serializer transition makes it reachable. On October 26, Windows PHP release packages were rebuilt with Brotli 1.2.1 (`-1` package suffix).

**P0 — current lock pin vulnerable:** Bun 1.4.2 includes the vulnerable Brotli decoder; the Nix `nodejs_24` patch version is not established. Update those separately. Avoid untrusted Brotli data until remediated and verified.

**P0 — TLS chain integrity for package downloads:** eMudhra's eight Global TLS intermediate CA keys were cryptographically weak and intermediates revoked. The vendor states **no infrastructure compromise and no data breach**; the security impact is that certain trust stores may accept certificates issued with weak intermediates. A TLS-path adversary is a prerequisite for impersonation; this is **not evidence that this repo, host or PyPI account was breached**. Updating host OS roots alone is insufficient for `pip`'s vendored certifi; `pip >=26.2.2`, `certifi >=2026.10.19` and `uv >=0.13.4` need evaluation independently. Rebuild cached containers and consider already-created venvs and managed Python installations.

**P1 — Claude Code opt-in:** GHSA-5xjx-9qx6-jcrv: pre-2.1.251 Edit TOCTOU arbitrary user-writable out-of-workspace file write on macOS when attacker-controlled code can run in its Bash sandbox. Both inspected Nix package definitions are above this floor, but a machine's global Claude executable, plugins and custom MCP trust boundaries remain unverified. Update manual installs to current vendor version; audit `bypassPermissions`, auto-approve, plugin trust and hooks before enabling untrusted projects. Do not commit credentials or silently change user permission modes.

## Safe verification checklist (operator run on each actual host)

1. Record whether the environment uses Nix system profiles, Home Manager, nix-darwin, a separate installer, Docker, mise/FVM or other package managers. Updating flake.lock does **not** patch active software until rebuild/activation; a system daemon is not identified by a CLI version.
2. Record actual version outputs: `php --version` (only if PHP installed), `node --version`, `bun --version`, `uv --version`, `claude --version`, `python3 --version`, `python3 -m pip --version`. Obtain `certifi.__version__` by importing it **only in trusted environments**. Avoid sending complete env variables or configuration with API keys into incident tickets.
3. To read **evaluated** pinned versions before rebuilding, on the checkout with trusted/cached flake inputs use `nix eval --offline --raw --impure --expr 'let f = builtins.getFlake (toString ./.); p = f.inputs.nixpkgs.legacyPackages.${builtins.currentSystem}; in p.uv.version'`; separately evaluate `p.bun.version`, `p.nodejs_24.version`, `p.claude-code.version`. Note: this checks the system running the command; cross-system packages need separate evaluation. Offline evaluation may fail if inputs are not cached.
4. If not already operating with trusted roots, do not blindly run unauthenticated `curl | sh`, `pip install` or unverified downloads over suspect TLS. Follow the Python security notice for verified, hash-pinned pip bootstrapping or use a separately trusted release channel.
5. Create **a dedicated dependency update PR** with provenance-verified `flake.lock` revision/NAR hash and supported platform versions. A bump needs package build evaluation and macOS/Home Manager/nix-darwin compatibility; do not manually synthesize `narHash` or replace the nixpkgs input with an unverified floating reference.
6. Verify applicable CI checks **and** representative runtime on disposable macOS/Linux hosts; inspect deployed executables after activation, inspect Python venvs and container images separately, and record rollbacks. Security remediation is not complete if the pinned source is fixed but the active runtime remains vulnerable.

## Repository follow-up / gates

- **Draft #257**: updated `uv` resolves the *package definition* version floor but `bun` remains 1.4.2; keep release/activation review gate and pursue a follow-up fix instead of declaring all-clear.
- **Draft #244**: the installer pin is unrelated to PHP / emSign / Bun updates and does not upgrade existing Nix daemons. Keep previously documented lifecycle acceptance.
- **Phase A PR #259**: a read-only installer inventory may help in later host assessments but cannot certify Nix/Node/Python/Claude security.
- **Follow-up OpenSpec:** propose a deterministic **read-only runtime version policy/audit** with explicit unknowns and controlled update workflows, without changing default packages until approved.
- **Evidence needed before changing Nix pins:** stable package rev and `narHash`, platform expression versions, SBOM when available, original package hashes, latest-head Rust/Nix/bootstrap/macOS gates and release physical acceptance. No security-check bypasses or host-wide force updates.

**Operational decision:** Do not merge a dependency PR solely because the lockfile is newer. Mark the exact vulnerabilities closed only after verifying patched **installed** versions and relevant TLS trust stores; retain incident evidence if any suspicious downloads/traffic occurred.
