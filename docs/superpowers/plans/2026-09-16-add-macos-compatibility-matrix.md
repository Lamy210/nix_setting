# macOS Compatibility Matrix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace mutable macOS CI/release hosts with explicit stable compatibility/current lanes, preserve the existing `macos-check` contract through a fail-closed aggregator, and isolate Xcode 27 Public Preview to a non-PR canary.

**Architecture:** `.github/workflows/check.yml` will gain a two-lane `macos-stable` matrix and retain `macos-check` as the external aggregator contract. `release-artifact-check` and `.github/workflows/release.yml` will pin the shipping path to `macos-26` + Xcode 26.6. A separate preview workflow will exercise `xcode-27` only on `develop`, schedule, or manual dispatch. Contract behavior is locked in `tests/ci-scripts.bats` before workflow edits.

**Tech Stack:** GitHub Actions YAML, Bash/Bats, Nix/Home Manager/nix-darwin, Rust/Tauri 2, OpenSpec.

**Spec:** `openspec/changes/add-macos-compatibility-matrix/design.md`

## Global Constraints

- Stable compatibility lane: `macos-15` + `/Applications/Xcode_26.3.app/Contents/Developer`.
- Stable current/release lane: `macos-26` + `/Applications/Xcode_26.6.app/Contents/Developer`.
- `macos-check` MUST remain the stable matrix aggregator and MUST fail closed for failure/cancelled/skipped lane results.
- `macos-check` MUST remain outside `ci-required` and no branch-protection server setting changes are part of this change.
- `release-artifact-check`, macOS CLI tag release, and DMG tag release MUST use `macos-26` + Xcode 26.6.
- Xcode 27 Public Preview MUST NOT run on `pull_request` and MUST NOT be a dependency of `macos-check`, `ci-required`, or release jobs.
- Active macOS build/release jobs MUST NOT depend on `macos-latest`.
- Preserve existing release build scripts; only host/toolchain selection changes.

---

### Task 1: Lock the workflow contract with failing Bats tests

**Files:**
- Modify: `tests/ci-scripts.bats`

**Interfaces:**
- Consumes: existing `workflow_job_block()` helper.
- Produces: regression contract for `macos-stable`, `macos-check`, release pinning, no active `macos-latest`, and preview-canary isolation.

- [ ] **Step 1: Add failing tests**

Add tests equivalent to:

```bash
@test "stable macOS matrix pins supported runner and Xcode pairs" {
  workflow=.github/workflows/check.yml
  grep -q '^  macos-stable:$' "$workflow"
  block="$(workflow_job_block macos-stable)"
  echo "$block" | grep -q 'fail-fast: false'
  echo "$block" | grep -q 'os: macos-15'
  echo "$block" | grep -q 'xcode: /Applications/Xcode_26.3.app/Contents/Developer'
  echo "$block" | grep -q 'os: macos-26'
  echo "$block" | grep -q 'xcode: /Applications/Xcode_26.6.app/Contents/Developer'
  echo "$block" | grep -Fq "DEVELOPER_DIR: \${{ matrix.xcode }}"
}

@test "macos-check aggregates stable matrix fail-closed and stays out of ci-required" {
  macos_check="$(workflow_job_block macos-check)"
  echo "$macos_check" | grep -q 'needs: \[macos-stable\]'
  echo "$macos_check" | grep -q 'if:.*always()'
  echo "$macos_check" | grep -q 'needs.macos-stable.result'
  ci_required="$(workflow_job_block ci-required)"
  run grep -q 'macos-check' <<<"$ci_required"
  [ "$status" -ne 0 ]
}

@test "shipping macOS paths pin macos-26 and Xcode 26.6" {
  release=.github/workflows/release.yml
  release_artifact="$(workflow_job_block release-artifact-check)"
  echo "$release_artifact" | grep -q 'runs-on: macos-26'
  echo "$release_artifact" | grep -q 'DEVELOPER_DIR: /Applications/Xcode_26.6.app/Contents/Developer'
  grep -q 'os: macos-26' "$release"
  grep -q '/Applications/Xcode_26.6.app/Contents/Developer' "$release"
}

@test "active macOS build paths do not use macos-latest" {
  run grep -nE 'runs-on: macos-latest|os: macos-latest' .github/workflows/check.yml .github/workflows/release.yml
  [ "$status" -ne 0 ]
}

@test "xcode 27 canary is isolated from pull requests and required gates" {
  workflow=.github/workflows/macos-preview-canary.yml
  [ -f "$workflow" ]
  grep -q 'xcode-27' "$workflow"
  run grep -q 'pull_request:' "$workflow"
  [ "$status" -ne 0 ]
  run grep -q 'continue-on-error: true' "$workflow"
  [ "$status" -ne 0 ]
  run grep -q 'xcode-27' .github/workflows/check.yml .github/workflows/release.yml
  [ "$status" -ne 0 ]
}
```

- [ ] **Step 2: Run the contract test and verify RED**

Run: `nix run nixpkgs#bats -- tests/ci-scripts.bats`

Expected: FAIL because `macos-stable`, explicit release pins, and `macos-preview-canary.yml` do not exist yet.

- [ ] **Step 3: Commit the RED test**

```bash
git add tests/ci-scripts.bats
git commit -m "test: define macOS compatibility matrix contract"
```

### Task 2: Implement the stable macOS matrix and aggregator

**Files:**
- Modify: `.github/workflows/check.yml`
- Test: `tests/ci-scripts.bats`

**Interfaces:**
- Consumes: matrix rows `{os, xcode}`.
- Produces: `macos-stable` worker and external `macos-check` aggregator.

- [ ] **Step 1: Replace the old single `macos-check` worker**

Use this structure:

```yaml
  macos-stable:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: macos-15
            xcode: /Applications/Xcode_26.3.app/Contents/Developer
          - os: macos-26
            xcode: /Applications/Xcode_26.6.app/Contents/Developer
    runs-on: ${{ matrix.os }}
    env:
      DEVELOPER_DIR: ${{ matrix.xcode }}
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262
      - uses: cachix/install-nix-action@13d8dd58da0234aa297dedd986986ccb8e7f3e24
      - name: macOS / Xcode diagnostics
        run: |
          sw_vers
          uname -m
          printf 'DEVELOPER_DIR=%s\n' "$DEVELOPER_DIR"
          xcodebuild -version
          xcrun --sdk macosx --show-sdk-version
          xcrun --find clang
      - name: nix flake check
        run: nix flake check --allow-import-from-derivation
      - name: build macos home-manager
        run: nix build .#homeConfigurations.darwin-aarch64.activationPackage
      - name: build nix-darwin system
        run: nix build .#darwinConfigurations.darwin-aarch64.system

  macos-check:
    needs: [macos-stable]
    if: ${{ always() }}
    runs-on: ubuntu-latest
    steps:
      - name: require stable macOS matrix
        env:
          MACOS_STABLE: ${{ needs.macos-stable.result }}
        run: |
          set -euo pipefail
          printf 'macos-stable=%s\n' "$MACOS_STABLE"
          if [ "$MACOS_STABLE" != "success" ]; then
            echo "ERROR: stable macOS matrix did not succeed" >&2
            exit 1
          fi
```

- [ ] **Step 2: Run static and contract tests**

Run:

```bash
nix run nixpkgs#actionlint -- .github/workflows/check.yml
nix run nixpkgs#bats -- tests/ci-scripts.bats
```

Expected: stable-matrix and aggregator contract tests PASS; release/canary tests still FAIL.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/check.yml tests/ci-scripts.bats
git commit -m "ci: add stable macOS compatibility matrix"
```

### Task 3: Pin PR and tag release macOS hosts/toolchains

**Files:**
- Modify: `.github/workflows/check.yml`
- Modify: `.github/workflows/release.yml`
- Test: `tests/ci-scripts.bats`

**Interfaces:**
- Produces: shipping contract `macos-26` + Xcode 26.6.

- [ ] **Step 1: Pin PR release artifact job**

Change to:

```yaml
  release-artifact-check:
    runs-on: macos-26
    env:
      DEVELOPER_DIR: /Applications/Xcode_26.6.app/Contents/Developer
```

Add a diagnostic step before building artifacts:

```yaml
      - name: macOS / Xcode diagnostics
        run: |
          sw_vers
          uname -m
          printf 'DEVELOPER_DIR=%s\n' "$DEVELOPER_DIR"
          xcodebuild -version
          xcrun --sdk macosx --show-sdk-version
          xcrun --find clang
```

- [ ] **Step 2: Pin release workflow CLI matrix row**

Replace the macOS row with:

```yaml
          - os: macos-26
            target: aarch64-darwin
            artifact: schneeforge-aarch64-darwin
            xcode: /Applications/Xcode_26.6.app/Contents/Developer
```

Set `DEVELOPER_DIR` only on macOS diagnostics/build/package steps so the Linux matrix row does not need an `xcode` value.

- [ ] **Step 3: Pin DMG release job**

```yaml
  build-dmg:
    runs-on: macos-26
    env:
      DEVELOPER_DIR: /Applications/Xcode_26.6.app/Contents/Developer
```

- [ ] **Step 4: Add release diagnostics and preserve build scripts unchanged**

Use `sw_vers`, `uname -m`, selected Xcode path, `xcodebuild -version`, `xcrun --sdk macosx --show-sdk-version`, and `xcrun --find clang`; do not change `scripts/ci/build-release-macos-cli.sh` or `scripts/ci/build-release-macos-dmg.sh`.

- [ ] **Step 5: Verify GREEN for release-pin tests**

Run:

```bash
nix run nixpkgs#actionlint -- .github/workflows/check.yml .github/workflows/release.yml
nix run nixpkgs#bats -- tests/ci-scripts.bats
```

Expected: stable and shipping pin tests PASS; canary test still FAIL.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/check.yml .github/workflows/release.yml tests/ci-scripts.bats
git commit -m "ci: pin macOS release toolchain"
```

### Task 4: Add isolated Xcode 27 preview canary

**Files:**
- Create: `.github/workflows/macos-preview-canary.yml`
- Create: `.github/actionlint.yaml` while actionlint lacks the hosted `xcode-27` label in its built-in list
- Test: `tests/ci-scripts.bats`

**Interfaces:**
- Produces: non-PR preview signal only; canary failures remain visibly red without blocking PR gates.

- [ ] **Step 1: Create the preview workflow**

```yaml
name: macos-preview-canary

on:
  push:
    branches: [develop]
  schedule:
    - cron: '17 3 * * *'
  workflow_dispatch:

permissions:
  contents: read

jobs:
  xcode-27-canary:
    runs-on: xcode-27
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262
      - name: macOS / Xcode diagnostics
        run: |
          sw_vers
          uname -m
          printf 'selected-xcode=%s\n' "$(xcode-select -p)"
          xcodebuild -version
          xcrun --sdk macosx --show-sdk-version
          xcrun --find clang
      - uses: cachix/install-nix-action@13d8dd58da0234aa297dedd986986ccb8e7f3e24
      - name: nix flake check
        run: nix flake check --allow-import-from-derivation
      - name: evaluate Darwin configuration
        run: |
          nix eval .#homeConfigurations.darwin-aarch64.activationPackage.drvPath
          nix eval .#darwinConfigurations.darwin-aarch64.system.drvPath
```

Register `xcode-27` in `.github/actionlint.yaml` as a known runner label until actionlint's built-in hosted-runner list catches up; do not globally suppress runner-label errors.

- [ ] **Step 2: Run actionlint and Bats**

Run:

```bash
nix run nixpkgs#actionlint -- .github/workflows/check.yml .github/workflows/release.yml .github/workflows/macos-preview-canary.yml
nix run nixpkgs#bats -- tests/ci-scripts.bats
```

Expected: all macOS matrix contract tests PASS.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/macos-preview-canary.yml .github/actionlint.yaml tests/ci-scripts.bats
git commit -m "ci: add Xcode 27 preview canary"
```

### Task 5: Synchronize repository status documentation

**Files:**
- Modify: `docs/STATUS.md`
- Modify: `AGENTS.md`
- Modify: `openspec/changes/add-macos-compatibility-matrix/tasks.md`

**Interfaces:**
- Produces: current-work documentation matching the implementation state.

- [ ] **Step 1: Update status/current-work text**

Record that `refactor-ci-critical-path` is archived, PR #93 is the active implementation PR, stable lanes are `macos-15/Xcode 26.3` and `macos-26/Xcode 26.6`, and Xcode 27 remains preview canary only.

- [ ] **Step 2: Mark completed implementation tasks**

Mark tasks 2.x through 5.x complete only after the corresponding workflow/test/document changes exist.

- [ ] **Step 3: Commit**

```bash
git add docs/STATUS.md AGENTS.md openspec/changes/add-macos-compatibility-matrix/tasks.md
git commit -m "docs: record macOS compatibility matrix rollout"
```

### Task 6: Verify PR behavior, cost, and merge safety

**Files:**
- Modify if needed: `openspec/changes/add-macos-compatibility-matrix/tasks.md`

**Interfaces:**
- Consumes: GitHub Actions results for PR #93.
- Produces: evidence for merge readiness.

- [ ] **Step 1: Verify latest-head static/spec gates**

Require success for `openspec-check`, `lint`, Bats contract step, and existing required contexts.

- [ ] **Step 2: Verify stable matrix**

Confirm both `macos-stable (macos-15, ...)` and `macos-stable (macos-26, ...)` are success and `macos-check` is success.

- [ ] **Step 3: Verify release artifact path**

Confirm `release-artifact-check` runs on `macos-26`, diagnostics report Xcode 26.6, and artifact scripts succeed.

- [ ] **Step 4: Verify preview isolation**

Confirm no `xcode-27-canary` job exists in the pull-request run and `ci-required` still aggregates exactly the existing seven contexts.

- [ ] **Step 5: Measure macOS runner cost**

Compare stable matrix worker elapsed totals to the previous single `macos-check` lane. If total persistently exceeds about 2.5x, do not declare completion; revise cadence or coverage.

- [ ] **Step 6: Final review and task sync**

Review the PR diff for accidental `macos-latest`, required-context drift, preview dependency, or release-script changes. Update tasks 6.1–6.6 with measured evidence.

- [ ] **Step 7: Mark PR ready and squash merge after latest-head checks are green**

Use the existing topic-branch policy; do not merge with pending/failing checks.

### Task 7: Archive the OpenSpec change after implementation merge

**Files:**
- Move: `openspec/changes/add-macos-compatibility-matrix/` → `openspec/changes/archive/2026-09-16-add-macos-compatibility-matrix/`
- Modify: `openspec/specs/development-workflow/spec.md`

**Interfaces:**
- Produces: archived implementation history and synced main spec.

- [ ] **Step 1: Create `chore/archive-add-macos-compatibility-matrix` from latest `develop`**
- [ ] **Step 2: Archive with spec sync and run strict validation**
- [ ] **Step 3: Open a separate archive PR to `develop`**
- [ ] **Step 4: Squash merge only after required checks are green**