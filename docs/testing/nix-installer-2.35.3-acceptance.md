# nix-installer 2.35.3 security update — acceptance and rollout

Tracking: [Draft PR #244](https://github.com/Lamy210/nix_setting/pull/244), [Actions PR policy #240](https://github.com/Lamy210/nix_setting/issues/240). Applies to the proposed new-install provider pin; **does not automatically update existing Nix installations**.

## Security rationale and provenance

NixOS/nix-installer [2.35.3 release](https://github.com/NixOS/nix-installer/releases/tag/2.35.3) (2026-10-14) bundles Nix 2.35.3 with security fixes described in [the October 2026 advisory](https://discourse.nixos.org/t/security-advisory-for-nix-and-lix-october-2026/80547). This is a **security-sensitive** provider update, not merely a routine lockfile refresh.

Upstream release assets must have both a matching SHA256 and verified SLSA provenance. The existing `upstream-nix-installer` workflow reported successful asset/provenance verification for 2.35.3. Official GitHub release asset digests were separately checked against #244:

| Asset | SHA256 |
| --- | --- |
| `nix-installer-x86_64-linux` | `59a1841db7d41d5127163cc8acad45a04b0d73e95a022269d4b9e7ae2ad7fab2` |
| `nix-installer-aarch64-linux` | `633bc7451945922e57f0912000f9876d0bfa001b92416dab853735be8cd64743` |
| `nix-installer-aarch64-darwin` | `c4991562a71a7ef2dc52d9760545c8541db4a468bbcc91dcafd57175638bf17e` |

These are release artifact hashes, **not** evidence that a host has been patched. Local runtime verification still matters.

## Acceptance gate 1: changes and CI

- [x] Scope: only `bootstrap-manifest.toml` provider version and the three corresponding digests before adding this documentation
- [x] 2.35.3 upstream release notes reviewed: legacy pre-2.35.2 `enable_flakes` receipt compatibility (#224), `systemctl status --no-pager` (#230), bundled Nix patch release
- [x] Official release asset hashes agree with proposed pin
- [ ] GitHub workflow SHA256SUMS and SLSA verification evidence reconfirmed against the exact target revision
- [ ] After syncing #244 with current `develop`, **all final-head** required and ancillary checks pass, including managed-nix Docker install/doctor/uninstall/reinstall and the special macOS bump lifecycle lane
- [ ] Explicitly check installed Nix version is 2.35.3 and verify `nix store ping` / flakes behavior in the fresh-install test output

## Acceptance gate 2: macOS aarch64 release cycle

Follow [macOS Final Acceptance](./macOS-final-acceptance-checklist.md) with a disposable fresh target and the **same release artifacts and manifest** that will ship. Record target tag, machine/OS, CLI SHA, provider binary SHA, CLI command results, receipt `version`, installer ownership hash and recovery/rollback observations.

- [ ] Fresh Mac: Nix absent before start; install plan and user confirmation reviewed before privileged action
- [ ] Install succeeds; `/nix/receipt.json` and `/nix/schneeforge-managed.json` valid
- [ ] Doctor and store ping work; `nix --version` confirms patched installed runtime
- [ ] Verified uninstall, reinstall and repair/recovery behavior (on **disposable** test machine only)
- [ ] Home Manager / nix-darwin apply, verify, rollback and Finder/GUI gates remain intact
- [ ] Owner/release reviewer signs off the candidate for the SchneeForge release cycle

Existing CI build/realization checks do **not** prove these physical activation gates have passed. Do not change branch protection, weaken tests or mark this section complete based on build-only CI.

## Acceptance gate 3: already-installed machines (separate change)

`bootstrap-manifest.toml` is embedded in the SchneeForge CLI for selecting binaries during **new installation**. `ManagedNix::prepare_plan` / `execute_plan` reject hosts with existing Nix markers, and `schneeforge nix uninstall` uses the original receipt and installer version.

- [ ] Define and review an **in-place, non-destructive Nix security upgrade** procedure for each supported host type and installation topology, with vendor-backed commands and rollback
- [ ] Record pre-upgrade Nix/daemon version, build/user group configuration, provider and ownership/receipt metadata
- [ ] Run the approved upgrade on a disposable clone/test host first and verify daemon/runtime version, store ping, flakes, Home Manager/nix-darwin configuration activation, installer receipt preservation, doctor and recovery path
- [ ] Communicate to existing users that merging #244 by itself will **not** patch their current installations

**Do not run `schneeforge nix uninstall --force` or delete `/nix` as an automated security-upgrade shortcut.** Force uninstall bypasses ownership/integrity safeguards and could destroy an otherwise healthy installation. Upgrade design and existing-user rollout are tracked separately from PR #244.

## Merge decision

Treat #244 as **REQUEST CHANGES / Draft** until both final-head CI and the documented release-cycle acceptance for new installations pass. Security urgency raises priority but is not a reason to bypass install/uninstall trust gates. Changes to existing-user upgrade code, if needed, belong to a separate reviewed change with dedicated regression tests.
