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
| `nodejs_24` | **24.21.0 observed in current-checkout CI Nix store** | **24.21.0** (verified from candidate `v24.nix`) | **Below patched 24.22.1 floor** for Node 24; prioritize a separately tested patch for root/standalone Node shells. The CI evidence does **not** establish what is active on every developer machine. Node 22/26 floors are 22.23.4/26.12.1. |
| `python3`, `pip`, `certifi` | Python/pip versions not established; Nixpkgs certifi definition 2026.06.17 with `cacert 3.126` | Python/pip versions not established; Nixpkgs certifi definition 2026.07.22 with **`cacert 3.131.1`** | Verify interpreter and installed/bundled CA stores individually, including `ensurepip` and existing venvs. Recommended patched Python lines: 3.11.18, 3.12.16, 3.13.18, 3.14.10, 3.15.2 (or newer supported patches); pip >=26.2.2, certifi >=2026.10.19. |
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

**P0 — current Nix CI runtime vulnerable:** Bun 1.4.2 includes the vulnerable Brotli decoder; the current-checkout root devshell run retrieved `nodejs-24.21.0` from Nix, below fixed **24.22.1**. The independent Node template and installed user machines still require their own evidence. Update Bun and Node separately. Avoid untrusted Brotli data until remediated and verified.

**P0 — TLS chain integrity for package downloads:** eMudhra's eight Global TLS intermediate CA keys were cryptographically weak and intermediates revoked. The vendor states **no infrastructure compromise and no data breach**; the security impact is that certain trust stores may accept certificates issued with weak intermediates. A TLS-path adversary is a prerequisite for impersonation; this is **not evidence that this repo, host or PyPI account was breached**. Updating host OS roots alone is insufficient for `pip`'s vendored certifi; `pip >=26.2.2`, `certifi >=2026.10.19` and `uv >=0.13.4` need evaluation independently. Rebuild cached containers and consider already-created venvs and managed Python installations.

**P1 — Claude Code opt-in:** GHSA-5xjx-9qx6-jcrv: pre-2.1.251 Edit TOCTOU arbitrary user-writable out-of-workspace file write on macOS when attacker-controlled code can run in its Bash sandbox. Both inspected Nix package definitions are above this floor, but a machine's global Claude executable, plugins and custom MCP trust boundaries remain unverified. Update manual installs to current vendor version; audit `bypassPermissions`, auto-approve, plugin trust and hooks before enabling untrusted projects. Do not commit credentials or silently change user permission modes.

## Safe verification checklist (operator run on each actual host)

1. Record whether the environment uses Nix system profiles, Home Manager, nix-darwin, a separate installer, Docker, mise/FVM or other package managers. Updating flake.lock does **not** patch active software until rebuild/activation; a system daemon is not identified by a CLI version.
2. Record actual version outputs: `php --version` (only if PHP installed), `node --version`, `bun --version`, `uv --version`, `claude --version`, `python3 --version`, `python3 -m pip --version`. Obtain `certifi.__version__` by importing it **only in trusted environments**. Avoid sending complete env variables or configuration with API keys into incident tickets.
3. To read **evaluated** pinned versions before rebuilding, on the checkout with trusted/cached flake inputs use `nix eval --offline --raw --impure --expr 'let f = builtins.getFlake (toString ./.); p = f.inputs.nixpkgs.legacyPackages.${builtins.currentSystem}; in p.uv.version'`; separately evaluate `p.bun.version`, `p.nodejs_24.version`, `p.claude-code.version`. Note: this checks the system running the command; cross-system packages need separate evaluation. Offline evaluation may fail if inputs are not cached.
4. If not already operating with trusted roots, do not blindly run unauthenticated `curl | sh`, `pip install` or unverified downloads over suspect TLS. Follow the Python security notice for verified, hash-pinned pip bootstrapping or use a separately trusted release channel.
5. Create **a dedicated dependency update PR** with provenance-verified `flake.lock` revision/NAR hash and supported platform versions. A bump needs package build evaluation and macOS/Home Manager/nix-darwin compatibility; do not manually synthesize `narHash` or replace the nixpkgs input with an unverified floating reference.
6. Verify applicable CI checks **and** representative runtime on disposable macOS/Linux hosts; inspect deployed executables after activation, inspect Python venvs and container images separately, and record rollbacks. Security remediation is not complete if the pinned source is fixed but the active runtime remains vulnerable.

## Deep-dive evidence: template locks, CA trust, host-shadowed executables

### Separate nixpkgs pins (confirmed from repository and pinned nixpkgs definitions)

Draft PR #257 was additionally inspected: its nixpkgs `3ec636362e1816bc2f0888fa0fa2c69f1bc9e504` changes `cacert` to NSS **3.131.1** and `uv` to **0.13.4**, but leaves **Node 24.21.0, Bun 1.4.2, shared Brotli 1.2.0 and c-ares 1.34.8**. This is useful partial CA mitigation after host activation, **not** a complete Node/Bun fix; #257 retains real-host acceptance gates and is not presumed merged.

The independent Node/Python templates contain their **own checked-in flake.lock**, `nixpkgs@2fcb964de67fcf60b43471c55d5d99e61a9ccb5a` (older than root `fe1324228`). CI copies each template away from this checkout, so it **does not use the root flake.lock** for those tests.

| Executable / trust material | Root pin `fe1324228` | Node/Python template pin `2fcb964d` | Exposure |
| --- | --- | --- | --- |
| `nodejs_24` | 24.21.0 | **24.18.1** | Both older than 24.22.1. Node 24 patch set fixes Brotli, TLS hostname validation, DNS, eMudhra root, c-ares, OpenSSL and Undici; **a Bun-only patch does not fix Node**. |
| `bun` | 1.4.2 | **1.3.13** | Both older than 1.4.4. PR #264 **merged** with checksum-verified Bun 1.4.4 overrides in both Node shells, but only after actual rebuilt shell activation. |
| `uv` | 0.13.0 | **0.12.1** | Both older than 0.13.4. PR #266 proposes 0.13.5 in both Python shells; the Rust vendor rebuild took ~23 min in first successful build and initial CI incorrectly required `uv --version` to omit its architecture suffix. |
| `python3Packages.certifi` | 2026.06.17 | 2026.06.17 | **Nixpkgs patches this package to symlink its `certifi/cacert.pem` to the `cacert` derivation**; upstream PyPI certifi's version floor alone is not sufficient evidence for Nix's effective CA trust. Inspect `certifi.where()` / symlink target and CA fingerprints, not only `certifi.__version__`. |
| `cacert` NSS data | 3.126 | 3.126 | Mozilla's NSS 3.131.1 is the specifically documented root-store update for the compromised eMudhra intermediates. These two locked definitions predate it. Determine effective trust after activation; if patched `cacert` cannot be deployed immediately, use an independently verified trust-store mitigation without disabling certificate validation. |

**Verified Nixpkgs source links:**
- [Older template Node](https://github.com/NixOS/nixpkgs/blob/2fcb964de67fcf60b43471c55d5d99e61a9ccb5a/pkgs/development/web/nodejs/v24.nix), [Bun](https://github.com/NixOS/nixpkgs/blob/2fcb964de67fcf60b43471c55d5d99e61a9ccb5a/pkgs/by-name/bu/bun/package.nix), [uv](https://github.com/NixOS/nixpkgs/blob/2fcb964de67fcf60b43471c55d5d99e61a9ccb5a/pkgs/by-name/uv/uv/package.nix).
- [Pinned certifi CA-bundle substitution](https://github.com/NixOS/nixpkgs/blob/fe13242281a67641c0102cdb1087f0ebe586c7df/pkgs/development/python-modules/certifi/default.nix) and [NSS 3.126 CA definition](https://github.com/NixOS/nixpkgs/blob/fe13242281a67641c0102cdb1087f0ebe586c7df/pkgs/by-name/ca/cacert/package.nix).
- [Upstream patched NSS 3.131.1 definition](https://github.com/NixOS/nixpkgs/blob/master/pkgs/by-name/ca/cacert/package.nix); [Mozilla response](https://blog.mozilla.org/security/2026/10/20/protecting-firefox-users-following-the-compromise-of-emudhra-intermediate-certificates/).

### Runtime authority is split; a Nix PR is not a host-wide fix

The repo's `docs/runtime-ownership.md` delegates **global Node** to `mise`, with `config/mise/config.toml` declaring `node = "lts"` (a moving selector), and sets `BUN_INSTALL=$HOME/.bun` plus appends its bin directory to the user's shell PATH in `modules/shell.nix`. Therefore the effective `node` and `bun` are determined by PATH and shell startup, **not merely the Nix input**. For each shell record `command -v`, `type -a` (interactive), actual version, resolved binary path and package-manager ownership. A patched `nix develop .#node` does not patch `$HOME/.bun/bin/bun`.

The `templates/devenv/flake.nix` separately pulls `nodejs_24`; this template was **not** covered by PR #264's Bun-specific patch. Evaluate its own flake lock/resolved input and build closure before declaring any repo-wide Node remediation.

### Certificate trust boundaries, verified behaviors

- The eMudhra incident involved eight weak intermediate CA private keys, **not a reported data breach**. Mozilla distributed OneCRL denylisting to ordinary Firefox users on October 17 and removed the affected root in NSS 3.131.1; Chrome used a CRLSet SPKI block. Those browser-specific rapid mitigations **do not automatically imply CLI HTTP clients, custom Node bundles, Python trust stores or older containers are patched**.
- The Python community recommends `certifi >=2026.10.19`, `pip >=26.2.2`, `uv >=0.13.4`, `sigstore >=4.5.1`, `conda >=26.9.4`, `pixi >=0.82.1` and `virtualenv >=21.14.12`. Each may embed or vendor CA data independently; inspect the **specific executable / venv / container layer**.
- Cloudflare reports that default **Workers Builds and Pages build images** were updated October 23 with newer CA roots, Node 24.22.1, Bun 1.4.4 and pip 26.2.2. Overrides `BUN_VERSION`, `SSL_CERT_FILE`, `PIP_CERT`, `REQUESTS_CA_BUNDLE` and `NODE_OPTIONS=--use-bundled-ca` can reintroduce trust issues. This says nothing about the TLS trust of arbitrary self-hosted deployments.
- **Never** suggest `curl -k`, `NODE_TLS_REJECT_UNAUTHORIZED=0`, `pip --trusted-host` or disabling hostname validation as incident mitigation.

### Why naive Node version overrides are insufficient

Pinned `nodejs_24` derives from Nixpkgs' `v24.nix`, whose upstream version changed from **24.21.0 to 24.22.1** in merged Nixpkgs PR #576777. But the Node derivation's `nodejs.nix` links a separately packaged `brotli` and `c-ares`. Existing pinned Nixpkgs uses `brotli 1.2.0` and `c-ares 1.34.8`, so just changing the Node source hash and version is **not sufficient** to prove the dynamically linked closure is patched. Nixpkgs PR #579225 documented applying the [Brotli patch](https://github.com/google/brotli/commit/0ed4268a4401abcb4472096340477cf4af7d1a2b) and upgrading shared c-ares 1.34.9, alongside Node 24.22.1.

Treat source version, dynamically linked library versions, root CA set, build output and **the executed node binary** as separate evidence. Prefer a published complete upstream Nixpkgs security revision validated on macOS/Linux; otherwise isolate a reviewable source+shared-library+CA backport with exact hashes, trusted provenance, rebuild gates and rollback. The currently checked upstream master/unstable paths do not by themselves prove all components landed together.

Sources: [Node 24.22.1 official release](https://nodejs.org/en/blog/release/v24.22.1), [Nixpkgs Node update PR](https://github.com/NixOS/nixpkgs/pull/576777), [shared-library fix design #579225](https://github.com/NixOS/nixpkgs/pull/579225), [Bun advisories](https://github.com/oven-sh/bun/security/advisories/GHSA-663w-cv8g-jj2p).

### PHP2Shell and Claude Code follow-on conditions

- JPCERT/CC's October 26 update reports **claimed PoCs and attempted-exploitation traffic** for PHP2Shell, but does not establish that every PoC achieves RCE in every PHP runtime. For an exposed PHP installation, patch and review request/application logs, unexpected processes and persistence artifacts. Separate CVE-2026-115560 remains subject to an unusual `session.serialize_handler` transition and enabled session upload progress; disable `session.upload_progress.enabled` as the vendor-recommended short-term mitigation when applicable.
- Additional Claude Code advisories have independent patched floors: [plugin SHA-pin bypass >=2.1.179](https://github.com/anthropics/claude-code/security/advisories/GHSA-pq7j-f95f-qfcv), [workspace-write TOCTOU >=2.1.129](https://github.com/anthropics/claude-code/security/advisories/GHSA-5j29-h97v-84ch), [server-managed settings bypass >=2.1.260](https://github.com/anthropics/claude-code/security/advisories/GHSA-gfvf-j8jh-jxxw), [macOS Edit TOCTOU >=2.1.251](https://github.com/anthropics/claude-code/security/advisories/GHSA-5xjx-9qx6-jcrv). Both inspected optional Nix definitions (2.1.300 / 2.1.308) meet **these** floors. The Windows Claude Desktop [uncontrolled git.exe search-path advisory](https://github.com/anthropics/claude-code/security/advisories/GHSA-4p2j-qj3c-6x84) affects a **different product**, not the optional CLI module; verify Desktop separately.

### First-run bootstrap integrity is not authenticity under a hostile TLS trust root

The published `install.sh` is supported via **`curl | bash`**. It downloads both an executable asset and `CHECKSUMS.txt` over HTTPS from the same GitHub release trust boundary, then checks the matching SHA256. It also stages the binary into a root-owned directory and re-hashes it before `sudo` execution. This usefully prevents accidental corruption and a local post-download binary swap, but it **does not authenticate the remote publisher when the initial script or both asset and checksum responses are forged through a trusted-but-compromised TLS intermediary**. Cryptographic signing/attestation at release CI does not automatically mean the user's first-run downloader verifies it.

- This is a trust-boundary design finding, **not evidence that GitHub/SchneeForge/any actual user was compromised**. An on-path active attacker and a still-trusted certificate path (or otherwise controlled downloader) are required.
- Introduce an **independently established trust anchor** for first-run verification (trusted signed package/bundle, externally authenticated signature/public key, or signed release identities checked by already-trusted code), not verification solely coded in a freshly downloaded and potentially attacker-controlled shell script. Confirm repository/commit/workflow identity rather than only self-consistent SHA256.
- Until reviewed and implemented, do not claim `curl | bash` is safe against a hostile CA simply because a later binary is checked against `CHECKSUMS.txt`. Keep host TLS verification enabled; do not weaken or bypass `sudo` staging protections.
- Dedicated design work is tracked in [Issue #268](https://github.com/Lamy210/nix_setting/issues/268) and requires OpenSpec approval, backward compatibility assessment and per-platform release acceptance.

## Repository follow-up / gates

- **Draft #257**: updated `uv` resolves the *package definition* version floor but `bun` remains 1.4.2; keep release/activation review gate and pursue a follow-up fix instead of declaring all-clear.
- **Draft #244**: the installer pin is unrelated to PHP / emSign / Bun updates and does not upgrade existing Nix daemons. Keep previously documented lifecycle acceptance.
- **Phase A PR #259**: a read-only installer inventory may help in later host assessments but cannot certify Nix/Node/Python/Claude security.
- **Confirmed Node evidence:** PR #266 first root devShell CI downloaded `nodejs-24.21.0` (job https://github.com/Lamy210/nix_setting/actions/runs/40435042622/job/121961592785), independently of any release acceptance. Record current Node binary and upgrade to >=24.22.1.
- **Follow-up OpenSpec:** propose a deterministic **read-only runtime version policy/audit** with explicit unknowns and controlled update workflows, without changing default packages until approved.
- **Evidence needed before changing Nix pins:** stable package rev and `narHash`, platform expression versions, SBOM when available, original package hashes, latest-head Rust/Nix/bootstrap/macOS gates and release physical acceptance. No security-check bypasses or host-wide force updates.

**Operational decision:** Do not merge a dependency PR solely because the lockfile is newer. Mark the exact vulnerabilities closed only after verifying patched **installed** versions and relevant TLS trust stores; retain incident evidence if any suspicious downloads/traffic occurred.
