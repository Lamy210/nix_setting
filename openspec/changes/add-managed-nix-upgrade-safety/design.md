# Managed Nix upgrade safety — Design

## 1. Goal, assumptions, limits

- **Goal:** before any attempt to mutate a host's Nix, produce a reproducible, explicit read-only inventory and indicate whether the upgrade route is known/unknown. Avoid claiming vulnerability remediation from a provider bump.
- **Known:** SchneeForge owns optional `/nix/schneeforge-managed.json`; upstream receipt `/nix/receipt.json` is source of truth. Current macOS repo config declares `nix.enable = true`.
- **Assumptions requiring device validation:** daemon-version extraction across Nix 2.28/2.30/2.31/2.34/2.35; installed platform manager (nix-darwin, NixOS, other); actual running `nix-daemon`; rollback semantics on macOS.
- **Phase A:** zero root-only mutations, zero network, no secrets. Unexpected errors are reported, not silently turned into healthy/missing.
- **Nonfunctional targets (proposal):** no writes, no downloads, bounded subprocess timeout (10 s each), bounded stdout/stderr and no secrets in logs; deterministic JSON schema version 1; Linux/macOS CLI interoperability. Timeouts are acceptance targets, not measured current performance.

## 2. Options and decision

| Option | Benefits | Risks | Decision |
|---|---|---|---|
| A. Documentation-only vendor commands | Low implementation cost | No machine-readable inventory; users can choose incorrect updater | Insufficient alone |
| B. Read-only inspection, owner policy and later controlled update | Testable, non-destructive, lets us fail closed | Extra phase before patch automation | **Select Phase A** |
| C. Immediately run `sudo nix upgrade-nix` | Short path on eligible profile-managed hosts | Nix-darwin/NixOS/daemon ambiguity; unsigned/unchecked target, rollback unknown | Reject |

## 3. Boundaries / ownership

1. **Core `upgrade_inspect`** collects observations from injected resolvers and filesystem probes; `CLI` renders text or JSON, has no independent policy logic.
2. **Observed installation owner** is not inferred solely from checked-out Nix config. Record separate `repo_declares_nix_darwin` and `host_manager`. A declaration is evidence of intended configuration, *not proof of currently active host ownership*.
3. `OwnershipRecord::load` validates the JSON schema, provider label, stored checksum *format*, and recorded receipt path. It does **not** authenticate the record's filesystem provenance, current Nix package, daemon or binary signature. Name this result `ownership_record_valid` (not `schneeforge-verified`); even a structurally valid record cannot by itself authorize an upgrade. Receipt errors are distinguished: missing, parse error, permission denied, other I/O. Never interpret an invalid record as unmanaged.
4. Tool paths are resolved with `ToolInventory` using OS-native `PathBuf`, not shell strings or PATH-only. Direct `Command` subprocess with fixed argv, sanitized external output; no command interpolation.
5. `client_version` comes from resolved `nix --version` (strict prefix/semver parse only if recognized). For verified Nix versions, probe local daemon reachability with `nix store info --store daemon` (documented in Nix 2.35.2) or legacy `nix store ping --store daemon` where applicable; select a known command according to **observed CLI capabilities**, not unvalidated assumptions. These commands confirm reachability, **not daemon version**; on success the 2.35.2 `store info` manual documents exit 0 with no output. Keep `daemon_version` unknown unless an independently verified version probe is implemented. Do not copy `client_version` to it.
6. Nix `2.35.2` detection is informational only: `client_version` may match a vulnerable build, but remediation policy must consider daemon and installation manager. Unknown remains Unknown instead of claiming patched/secure.
7. Serialized report must be schema-versioned and machine-readable; confidence and reasons are explicit. Structured result distinguishes `unknown` / `unsupported` / `manual` / `not-installed`. Reports must not leak usernames, `HOME`, environment variables, auth secrets or receipt contents.

## 4. Phase A proposed contract

CLI: `schneeforge nix upgrade inspect [--json]` (new command; no updater invocation). Exit 0 only if observations completed without unexpected failure; return non-zero on inspection error, even if JSON error result is produced. A discovered unsupported installation is a **successful inspection**, not an execution grant.

Suggested report shape:
```json
{
  "schema_version": 1,
  "platform": "macos-aarch64",
  "client_version": {"state": "known", "value": "2.35.2"},
  "daemon_reachable": {"state": "known", "value": true},
  "daemon_version": {"state": "unknown", "reason": "stable-probe-unverified"},
  "installation_ownership": "schneeforge-record-valid",
  "host_manager": "unknown",
  "upgrade_route": "manual",
  "automation_eligible": false,
  "reasons": ["host-manager-not-verified", "daemon-version-unknown"]
}
```
This is a **sample shape, not a sample measured host**; actual schema fields and labels require implementation review. `schneeforge-record-valid` means structural consistency only, **not authenticated authority** or update eligibility. Avoid values borrowed from the sample when reporting a real host.

## 4.1. Design review: confirmed trust and observation boundaries

- **Confirmed — record authenticity:** `OwnershipRecord::load` parses JSON and validates declared fields (including 64-character hex checksum format) but does not inspect the actual cache binary or prove which process wrote the record. The initial report must say `record-valid`, never `owner-authenticated`.
- **Confirmed — daemon reachability is not its version:** Official Nix 2.35.2 docs describe `nix store info --store daemon` as a store-access probe with no output when successful. Never parse daemon version from its success or from the Nix CLI version.
- **Confirmed — nix-darwin settings can own the daemon:** upstream nix-darwin `nix.enable` controls system Nix, nix-daemon and `/etc/nix/nix.conf`. A checked-out `nix.enable = true` alone does not attest which generation is activated on this host.
- **Likely — inherited Nix environment can redirect the daemon:** Nix's `daemon` store may honor `NIX_DAEMON_SOCKET_PATH` and other store/environment overrides. An implementation claiming a *local* check must explicitly classify or reject nondefault socket overrides and never connect to an arbitrary remote store. A reachable socket alone does not establish host-manager identity.
- **Design constraint — read-only is not zero side effects:** Even a diagnostic CLI may write Nix caches/logs or trigger lazy daemon launch depending on host configuration. Phase A MUST avoid intentionally mutating configuration/store/profile/receipt, forbid downloads and remote endpoints, and record the limitations; claiming mathematically zero filesystem writes by every downstream subprocess would be unverified. Test explicit no-write contracts at SchneeForge boundaries.
- **Operational risk:** `std::process::Command::output` is unbounded and may leak uncontrolled stderr; use a bounded observation runner with timeout, maximum capture, output suppression/redaction and child cleanup, without shell interpolation. Permit ordinary timeout/unsupported statuses as structured unknowns; reserve nonzero CLI exit for malformed invocation or unexpected inspection failure.

## 5. Phase B gate (future change, NOT authorized here)

Before any mutating command:
- Detect effective management: `nix-darwin` (host-managed `nix.enable = true`), NixOS, Lix, Determinate, NixOS/nix-installer-managed standalone Nix, custom profile. Refuse unknown/foreign owner and mixed installations.
- Pin a specific security-fixed Nix artifact per supported release line (never equate nix-installer 2.35.3 to runtime Nix 2.35.3); establish independently rooted trust via authenticated source + content-addressed store signature/attestation, and verify store-path URL/trust keys. An expected SHA from the same untrusted origin is insufficient.
- Ask for explicit confirmation of exact target and affected profile. Recheck receipt, ownership, nix binary path, daemon and target after authorization to mitigate TOCTOU.
- Acquire a root-owned cross-process update lock separate from existing StateStore lock; avoid lock inversion with configuration apply/release.
- Persist append-only transaction with original profile generation, upstream receipt identity/digest and selected trusted store path; do not rewrite receipt or ownership as proof of success.
- Stage target, verify signatures/closure, switch owner-controlled profile using its documented supported mechanism; reload the actual daemon only when manager supports it. For nix-darwin owner, prefer dedicated declarative update path and run controlled activation; never implicitly run `nix upgrade-nix`.
- Require independently probed post-upgrade CLI and daemon versions, `nix store ping`, flake evaluation and ownership/uninstall continuity; treat version mismatch as incomplete.
- Rollback is a separate explicit action, only if the original generation can be identified and verified; interrupted or ambiguous update is degraded and **not retried automatically**. Never uninstall, delete `/nix`, or mutate build users as an upgrade shortcut.

## 6. Failure modes and mitigations

| Failure | Phase A behavior | Phase B requirement |
|---|---|---|
| Receipt malformed / unreadable | Preserve distinct state; inspection fails or becomes unknown | Never mutate |
| Ownership absent / invalid | Unknown or foreign owner | Refuse mutation |
| `nix` not resolved | Not installed vs markers present differentiated | Refuse mutation |
| CLI version differs from daemon | Both independently reported; no safe status inference | Stop / require remedial action |
| Daemon unreachable / probe format changed | Unknown, error and reason | Fail closed |
| macOS nix-darwin declares Nix | Disclosure only; effective owner unverified | Use declarative owner, not raw profile update |
| Profile switch interrupted | No mutation | Journal, generation recovery, explicit rollback |
| Target signature mismatch | No download | Verify before switch; reject |
| Clock/network unavailable | Offline/read-only inspect | Verified pinned cached target or abort |

## 7. Security, operations and costs

- Threat model: privileged installer command injection, PATH spoofing, malicious substitute, stale receipt, partial update, conflicting owner, rollback to vulnerable version, sensitive stdout. Phase A adds no privilege or network entry points.
- Logging: structured action, duration, probe state, error category and decision reasons. Never log tokens, full env or receipt payload. Metrics (later): inspect failure by category, owner unknown ratio, upgrade abort, recovery failures; do not send automatically.
- Availability/DR: Phase A cannot make the host unavailable by design. Phase B requires disposable host recovery acceptance (restore verified generation and daemon; no `/nix` deletion). SLO for Phase B to be defined after measuring E2E.
- Costs: Phase A local commands + bounded time, no service infrastructure. Phase B egress/store closure download, CI disposable macOS/Linux runner time, disk for old/new store paths, signing/attestation verification are dominant variables. Monetary amount **unknown** until measured.

## 8. Rollout / verification

- Unit tests: missing/invalid/legacy receipt, old ownership, path with non-UTF-8 bytes, fake tool resolver, timeout, injected CLI version lines, daemon probe unknown and subprocess failures.
- CLI integration: macOS/Linux with and without Nix; no privileged writes, no network; JSON schema snapshot + semantic assertions.
- State invariants: immutable receipt, ownership, nix profile and daemon state; detect via snapshots before and after inspect. Run same observations as unprivileged user.
- CI: `openspec validate --all --strict`, Rust tests/clippy/fmt, Nix flake check, macOS 15/26; do not weaken existing gates.
- Phase B acceptance on disposable Linux multi-user and macOS aarch64 + nix-darwin: verified vendor-supported migration, old/new daemon version, unrelated flake apply, interrupted switch, bad signatures, original receipt/uninstall continuity, rollback, no destructive cleanup.

## 9. Primary sources and repository evidence

- https://github.com/NixOS/nix-installer/blob/main/README.md#upgrading-nix
- https://nix.dev/manual/nix/2.28/command-ref/new-cli/nix3-upgrade-nix
- https://nix.dev/manual/nix/2.35/command-ref/new-cli/nix3-store-info.html
- https://nix.dev/manual/nix/2.35/store/types/local-daemon-store.html
- https://github.com/nix-darwin/nix-darwin/blob/master/modules/nix/default.nix
- https://discourse.nixos.org/t/security-advisory-for-nix-and-lix-october-2026/80547
- `nix-darwin/config/nix-config.nix` (`nix.enable = true`)
- `crates/core/src/{diagnostics.rs,tool.rs,managed_nix/{mod.rs,receipt.rs,ownership.rs,status.rs}}`
- `crates/cli/src/nix_cmd.rs`, `docs/adr/0001-managed-nix-provider.md`
