#!/usr/bin/env bats

# scripts/ci/build-release-*.sh の gate logic を検証する。
# 実 binary build は CI job が担うため、ここでは grep pattern の
# 検出力 (false-negative / false-positive) を fixture で保証する。

# otool -L gate と同じ pattern。otool の dependency 行は行頭が
# indent されるため、^ 固定だと /nix/store 依存を見逃す (RC.2 follow-up)
NIX_STORE_PATTERN='^[[:space:]]*/nix/store/'

# build-release-macos-cli.sh の LC_RPATH 抽出 awk と同一の logic
extract_rpaths() {
  awk '
    $1 == "cmd" && $2 == "LC_RPATH" { in_rpath = 1; next }
    in_rpath && $1 == "path" { print $2; in_rpath = 0 }
  '
}

workflow_job_block() {
  local job="$1"
  awk -v header="  ${job}:" '
    $0 == header { in_job = 1; print; next }
    in_job && $0 ~ /^  [a-zA-Z0-9_-]+:$/ { exit }
    in_job { print }
  ' .github/workflows/check.yml
}

@test "otool gate pattern rejects indented /nix/store dependency" {
  output="$(printf 'result/bin/schneeforge:\n\t/nix/store/xxxx-libfoo.dylib (compatibility version)\n' \
    | grep -E "$NIX_STORE_PATTERN")"
  [ -n "$output" ]
}

@test "otool gate pattern rejects non-indented /nix/store dependency" {
  output="$(printf '/nix/store/xxxx-libfoo.dylib\n' | grep -E "$NIX_STORE_PATTERN")"
  [ -n "$output" ]
}

@test "otool gate pattern allows system libSystem dependency" {
  run sh -c "printf 'result/bin/schneeforge:\n\t/usr/lib/libSystem.B.dylib\n' | grep -E '$NIX_STORE_PATTERN'"
  [ "$status" -ne 0 ]
}

# LC_RPATH gate: @rpath 依存 + LC_RPATH /nix/store の組合せは
# otool -L には /nix/store が現れないため -l での抽出が必須
@test "LC_RPATH extraction rejects /nix/store rpath" {
  output="$(printf 'Load command 12\n      cmd LC_RPATH\n      cmdsize 32\n      path /nix/store/xxxx-libfoo/lib (offset 12)\n' \
    | extract_rpaths | grep '^/nix/store/')"
  [ -n "$output" ]
}

@test "LC_RPATH extraction allows /usr/local/lib rpath" {
  output="$(printf 'Load command 12\n      cmd LC_RPATH\n      cmdsize 32\n      path /usr/local/lib (offset 12)\n' \
    | extract_rpaths)"
  [ "$output" = "/usr/local/lib" ]
}

# readelf INTERP gate と同等の検査 (Linux static binary)
@test "INTERP gate pattern detects dynamic interpreter segment" {
  output="$(printf '  INTERP    0x0000000000000318\n' | grep -q INTERP && echo matched)"
  [ "$output" = "matched" ]
}

@test "INTERP gate pattern passes when no INTERP segment" {
  run sh -c "printf '  LOAD    0x0000000000000000\n' | grep -q INTERP"
  [ "$status" -ne 0 ]
}

# --- check-macos-portability.sh の arm64 判定 (RC.4 DMG 事故で追加) ---

@test "arm64 gate accepts arm64 architecture string" {
  output="$(printf 'arm64\n' | grep -qE '^(arm64|\*arm64)' && echo matched)"
  [ "$output" = "matched" ]
}

@test "arm64 gate rejects x86_64 architecture string" {
  run sh -c "printf 'x86_64\n' | grep -qE '^(arm64|\*arm64)'"
  [ "$status" -ne 0 ]
}

# --- build-release-macos-dmg.sh の pin 検証 ---

@test "tauri CLI sha256 pin is exact length" {
  SHA="$(grep '^TAURI_CLI_SHA256=' scripts/ci/build-release-macos-dmg.sh | cut -d'"' -f2)"
  [ "${#SHA}" -eq 64 ]
}

@test "tauri CLI download URL embeds pinned version" {
  VERSION="$(grep '^TAURI_CLI_VERSION=' scripts/ci/build-release-macos-dmg.sh | cut -d'"' -f2)"
  URL="$(grep '^TAURI_CLI_URL=' scripts/ci/build-release-macos-dmg.sh | sed "s/\${TAURI_CLI_VERSION}/$VERSION/" | cut -d'"' -f2)"
  echo "$URL" | grep -q "tauri-cli-v${VERSION}/"
}

@test "dmg script gates mounted app binary not raw build output" {
  grep -q 'hdiutil attach' scripts/ci/build-release-macos-dmg.sh
  grep -q 'check-macos-portability.sh' scripts/ci/build-release-macos-dmg.sh
  grep -q 'CFBundleShortVersionString' scripts/ci/build-release-macos-dmg.sh
}

# --- DMG 内 CLI sidecar (GUI escalation 先) の gate ---

@test "dmg script builds CLI before tauri bundle (externalBin source)" {
  # build script は target/<profile>/schneeforge を stage 元にするため、
  # tauri build の前に CLI build が必要
  CLI_BUILD_LINE="$(grep -n 'cargo build --release -p schneeforge' scripts/ci/build-release-macos-dmg.sh | cut -d: -f1)"
  TAURI_BUILD_LINE="$(grep -n 'TAURI_BIN. build' scripts/ci/build-release-macos-dmg.sh | cut -d: -f1 | head -1)"
  [ -n "$CLI_BUILD_LINE" ]
  [ -n "$TAURI_BUILD_LINE" ]
  [ "$CLI_BUILD_LINE" -lt "$TAURI_BUILD_LINE" ]
}

@test "dmg script verifies CLI sidecar inside mounted app" {
  # tauri 2.x は bundle 時に triple suffix を除去する
  grep -q 'MacOS/schneeforge-cli' scripts/ci/build-release-macos-dmg.sh
}

# scripts/ci/slsa_predicate.py の出力構造検証 (add-release-attestation-bundles)。
# cosign 署名は OIDC 依存で PR CI では実行できないため、純粋 logic である
# predicate 生成のみ dummy 値で検証する (D6)
DUMMY_SHA="0123456789abcdef0123456789abcdef01234567"

@test "slsa predicate contains builder id, materials sha1 and entrypoint" {
  tmp="$(mktemp -d)"
  python3 scripts/ci/slsa_predicate.py v9.9.9-rc.9 "$DUMMY_SHA" "refs/tags/v9.9.9-rc.9" "$tmp/predicate.json"
  python3 - "$tmp/predicate.json" "$DUMMY_SHA" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    p = json.load(f)
assert p["builder"]["id"] == "https://github.com/Lamy210/nix_setting/.github/workflows/release.yml@refs/tags/v9.9.9-rc.9", p["builder"]
assert p["materials"][0]["digest"]["sha1"] == sys.argv[2], p["materials"]
assert p["invocation"]["configSource"]["entryPoint"] == ".github/workflows/release.yml"
assert p["invocation"]["configSource"]["digest"]["sha1"] == sys.argv[2]
PY
}

@test "slsa predicate rejects malformed source sha" {
  tmp="$(mktemp -d)"
  run python3 scripts/ci/slsa_predicate.py v9.9.9 "not-a-sha" "refs/tags/v9.9.9" "$tmp/predicate.json"
  [ "$status" -ne 0 ]
}

# --- check.yml critical-path contract (refactor-ci-critical-path) ---

@test "rust required check fans out to two workers and aggregates fail-closed" {
  workflow=.github/workflows/check.yml
  grep -q '^  rust-quality:$' "$workflow"
  grep -q '^  rust-build-smoke:$' "$workflow"
  run grep -q '^  rust-cli-smoke:$' "$workflow"
  [ "$status" -ne 0 ]
  run grep -q '^  rust-desktop-smoke:$' "$workflow"
  [ "$status" -ne 0 ]
  grep -q '^  rust-check:$' "$workflow"

  rust_check_block="$(workflow_job_block rust-check)"
  echo "$rust_check_block" | grep -q 'needs: \[rust-quality, rust-build-smoke\]'
  echo "$rust_check_block" | grep -q 'if:.*always()'
  echo "$rust_check_block" | grep -q 'needs.rust-quality.result'
  echo "$rust_check_block" | grep -q 'needs.rust-build-smoke.result'
}

@test "build smoke owns Tauri deps and uses desktop compile gate" {
  workflow=.github/workflows/check.yml
  [ "$(grep -c 'libwebkit2gtk-4.1-dev' "$workflow")" -eq 1 ]
  build_block="$(workflow_job_block rust-build-smoke)"
  quality_block="$(workflow_job_block rust-quality)"
  echo "$build_block" | grep -q 'libwebkit2gtk-4.1-dev'
  echo "$build_block" | grep -q 'cargo build --release -p schneeforge'
  echo "$build_block" | grep -q 'cargo check --release --manifest-path apps/desktop/src-tauri/Cargo.toml'
  run grep -q 'cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml' <<<"$build_block"
  [ "$status" -ne 0 ]
  run grep -q 'libwebkit2gtk-4.1-dev' <<<"$quality_block"
  [ "$status" -ne 0 ]
}

@test "flake gate evaluates developer profile and realizes minimal profile" {
  flake_block="$(workflow_job_block flake-check)"
  echo "$flake_block" | grep -q 'nix flake check --allow-import-from-derivation'
  echo "$flake_block" | grep -q 'nix eval .#homeConfigurations.linux.activationPackage.drvPath'
  echo "$flake_block" | grep -q 'nix eval .#homeConfigurations.linux-arm.activationPackage.drvPath'
  echo "$flake_block" | grep -Fq "nix build .#homeConfigurations.linux.activationPackage --override-input profile \"path:\$PWD/tests/fixtures/profile-minimal.nix\""
  grep -q 'profile = "minimal"' tests/fixtures/profile-minimal.nix
  run grep -Eq 'run: nix build \.#homeConfigurations\.linux\.activationPackage$' <<<"$flake_block"
  [ "$status" -ne 0 ]
}

@test "shadow ci-required aggregates the existing seven required contexts" {
  ci_required_block="$(workflow_job_block ci-required)"
  [ -n "$ci_required_block" ]
  echo "$ci_required_block" | grep -q 'if:.*always()'
  for job in openspec-check flake-check rust-check lint bootstrap-test managed-nix-e2e release-artifact-check; do
    echo "$ci_required_block" | grep -q "needs.$job.result"
  done
}

# --- macOS compatibility matrix contract (add-macos-compatibility-matrix) ---

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

# --- Windows portability contract (add-windows-wsl2-platform) ---

@test "windows-check is pinned, hermetic and non-required" {
  workflow=.github/workflows/check.yml
  grep -q '^  windows-check:$' "$workflow"

  windows_check="$(workflow_job_block windows-check)"
  echo "$windows_check" | grep -q 'runs-on: windows-2025'
  echo "$windows_check" | grep -q 'cargo check --workspace'
  echo "$windows_check" | grep -q 'cargo test -p schneeforge-core'
  echo "$windows_check" | grep -q 'cargo test -p schneeforge --bin schneeforge'
  echo "$windows_check" | grep -q 'cargo test -p schneeforge --test windows_cli_contract'
  echo "$windows_check" | grep -q 'schneeforge.exe --version'
  echo "$windows_check" | grep -q 'schneeforge.exe --help'

  ci_required="$(workflow_job_block ci-required)"
  run grep -q 'windows-check' <<<"$ci_required"
  [ "$status" -ne 0 ]

  release_artifact="$(workflow_job_block release-artifact-check)"
  run grep -q 'windows-check' <<<"$release_artifact"
  [ "$status" -ne 0 ]
  run grep -q 'windows-check' .github/workflows/release.yml
  [ "$status" -ne 0 ]
}

@test "real WSL canary is isolated and exercises launcher transport" {
  workflow=.github/workflows/windows-wsl-canary.yml
  [ -f "$workflow" ]
  grep -q 'runs-on: windows-2025' "$workflow"
  grep -q 'schedule:' "$workflow"
  grep -q 'workflow_dispatch:' "$workflow"
  grep -q 'branches: \[develop\]' "$workflow"
  run grep -q 'pull_request:' "$workflow"
  [ "$status" -ne 0 ]
  run grep -q 'continue-on-error: true' "$workflow"
  [ "$status" -ne 0 ]
  grep -q 'wsl --install Ubuntu --no-launch --web-download' "$workflow"
  grep -q 'wsl --list --verbose' "$workflow"
  grep -q 'cargo build -p schneeforge' "$workflow"
  grep -Fq 'wsl -d Ubuntu --exec schneeforge __backend-info' "$workflow"
  [ "$(grep -Fc '[System.Diagnostics.ProcessStartInfo]::new()' "$workflow")" -eq 2 ]
  grep -Fq "\$startInfo.ArgumentList.Add('--wsl-distro')" "$workflow"
  grep -Fq "\$startInfo.ArgumentList.Add('Ubuntu')" "$workflow"
  grep -Fq "\$startInfo.ArgumentList.Add('canary-transport')" "$workflow"
  grep -Fq "\$startInfo.ArgumentList.Add('a b;\$(x)')" "$workflow"
  grep -Fq "\$startInfo.RedirectStandardOutput = \$true" "$workflow"
  grep -Fq "\$startInfo.RedirectStandardError = \$true" "$workflow"
  grep -Fq "\$startInfo.Environment['SCHNEEFORGE_WSL_DISTRO'] = 'Ubuntu'" "$workflow"
  grep -Fq "a b;\$(x)" "$workflow"
  grep -q '23' "$workflow"
  run grep -Fq 'Equivalent CLI selector contract: --wsl-distro Ubuntu' "$workflow"
  [ "$status" -ne 0 ]
  run grep -Fq '& .\target\debug\schneeforge.exe' "$workflow"
  [ "$status" -ne 0 ]
  run grep -q 'windows-wsl-canary' .github/workflows/check.yml .github/workflows/release.yml
  [ "$status" -ne 0 ]
}


# --- macOS Managed Nix lifecycle acceptance contract ---

@test "macOS Managed Nix lifecycle acceptance is manual isolated and release-pinned" {
  workflow=.github/workflows/macos-managed-nix-lifecycle.yml
  script=scripts/ci/macos-managed-nix-lifecycle.sh

  [ -f "$workflow" ]
  [ -f "$script" ]

  grep -q '^  workflow_dispatch:' "$workflow"
  grep -q 'runs-on: macos-15' "$workflow"
  grep -q 'run: bash scripts/ci/macos-managed-nix-lifecycle.sh' "$workflow"
  grep -q 'ACCEPTANCE_LOG_DIR' "$workflow"

  run grep -q 'pull_request:' "$workflow"
  [ "$status" -ne 0 ]
  run grep -q '^  push:' "$workflow"
  [ "$status" -ne 0 ]
  run grep -q '^  schedule:' "$workflow"
  [ "$status" -ne 0 ]
  run grep -q 'install-nix-action' "$workflow"
  [ "$status" -ne 0 ]

  grep -q 'GITHUB_ACTIONS' "$script"
  grep -q 'uname -m' "$script"
  grep -q 'cd "[$]WORK_DIR"' "$script"
  grep -q 'unset NIX_SETTING_DIR' "$script"
  grep -q 'sanitize_logs' "$script"
  grep -q 'CHECKSUMS.txt' "$script"
  grep -q 'shasum -a 256' "$script"
  grep -q 'nix install --yes' "$script"
  grep -q 'ExistingNixDetected' "$script"
  grep -q 'nix doctor' "$script"
  grep -q 'nix uninstall' "$script"
  grep -q 'reinstall' "$script"

  # Flakes capability smoke must be deterministic and must not consume the
  # unauthenticated GitHub API rate limit used by github: flake refs.
  grep -q 'local-flake-smoke' "$script"
  grep -q 'flake metadata "path:' "$script"
  run grep -q 'flake metadata "github:' "$script"
  [ "$status" -ne 0 ]

  # macOS may retain a bare synthetic /nix path after a successful uninstall.
  # Judge cleanup by runtime/state remnants instead of path existence alone.
  grep -q 'verify_uninstall_state' "$script"
  grep -q 'nix-mounted=' "$script"
  grep -q 'receipt-exists=' "$script"
  grep -q 'store-exists=' "$script"
  grep -q "\^_nixbld" "$script"
  grep -q 'org.nixos.nix-daemon' "$script"

  grep -q 'scripts/ci/macos-managed-nix-lifecycle.sh' .github/workflows/check.yml
  run grep -q 'macos-managed-nix-lifecycle.yml' .github/workflows/check.yml .github/workflows/release.yml
  [ "$status" -ne 0 ]
}


# --- Managed Nix bump macOS acceptance contract ---

@test "Managed Nix bump gets fresh-host macOS branch lifecycle acceptance" {
  workflow=.github/workflows/managed-nix-bump-acceptance.yml
  script=scripts/ci/macos-managed-nix-lifecycle.sh

  [ -f "$workflow" ]
  grep -q '^  pull_request:' "$workflow"
  grep -q 'bootstrap-manifest.toml' "$workflow"
  grep -q 'runs-on: macos-15' "$workflow"
  grep -q 'build-release-macos-cli.sh' "$workflow"
  grep -q -- '--local-binary' "$workflow"
  grep -q 'test ! -e /nix' "$workflow"
  grep -q '! command -v nix' "$workflow"
  grep -q 'ACCEPTANCE_LOG_DIR' "$workflow"

  # Acceptance must begin Nix-less; installing Nix before the lifecycle test
  # would invalidate the fresh-host contract.
  run grep -q 'install-nix-action' "$workflow"
  [ "$status" -ne 0 ]

  # The shared helper keeps release-tag verification and gains a local binary
  # mode for pre-release branch validation.
  grep -q 'MODE="release"' "$script"
  grep -q -- '--local-binary' "$script"
  grep -q 'using locally built branch CLI' "$script"
  grep -q 'release CLI SHA256 mismatch' "$script"
  grep -q 'Managed Nix branch lifecycle acceptance helper passed' "$script"
}


# --- Managed Nix bump manifest contract ---

@test "bootstrap manifest updater preserves comments and fails closed on schema drift" {
  script=scripts/ci/update-bootstrap-manifest.py
  [ -f "$script" ]

  tmp="$(mktemp -d)"
  manifest="$tmp/bootstrap-manifest.toml"
  cat >"$manifest" <<'EOF'
# retained header
# retained explanation

[managed_nix]
version = "2.35.1"

[managed_nix.sha256_by_arch]
x86_64-linux = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
aarch64-linux = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
aarch64-darwin = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
EOF

  python3 "$script" "$manifest" \
    --version 2.35.2 \
    --x86-64-linux 5448a1cd70ad945cb4d36365defbaf3731eba38e23859f3dc8bd7418e1946acc \
    --aarch64-linux a1b35e56da5adadbc117c3cf17b83948ac657f3c0bd79d47bbe0aa70832b5c8e \
    --aarch64-darwin 6314b195321b3acc6826b1c5d66bb9cf9306c8231c6dbb745f51a04c3bcee235

  grep -q '^# retained header$' "$manifest"
  grep -q '^# retained explanation$' "$manifest"
  grep -q '^version = "2.35.2"$' "$manifest"
  grep -q '^x86_64-linux = "5448a1cd70ad945cb4d36365defbaf3731eba38e23859f3dc8bd7418e1946acc"$' "$manifest"
  grep -q '^aarch64-linux = "a1b35e56da5adadbc117c3cf17b83948ac657f3c0bd79d47bbe0aa70832b5c8e"$' "$manifest"
  grep -q '^aarch64-darwin = "6314b195321b3acc6826b1c5d66bb9cf9306c8231c6dbb745f51a04c3bcee235"$' "$manifest"

  cp "$manifest" "$tmp/duplicate.toml"
  printf '%s\n' 'version = "9.9.9"' >>"$tmp/duplicate.toml"
  run python3 "$script" "$tmp/duplicate.toml" \
    --version 2.35.2 \
    --x86-64-linux 5448a1cd70ad945cb4d36365defbaf3731eba38e23859f3dc8bd7418e1946acc \
    --aarch64-linux a1b35e56da5adadbc117c3cf17b83948ac657f3c0bd79d47bbe0aa70832b5c8e \
    --aarch64-darwin 6314b195321b3acc6826b1c5d66bb9cf9306c8231c6dbb745f51a04c3bcee235
  [ "$status" -ne 0 ]
  echo "$output" | grep -q 'expected exactly one manifest field'

  grep -v '^aarch64-linux = ' "$manifest" >"$tmp/missing.toml"
  run python3 "$script" "$tmp/missing.toml" \
    --version 2.35.2 \
    --x86-64-linux 5448a1cd70ad945cb4d36365defbaf3731eba38e23859f3dc8bd7418e1946acc \
    --aarch64-linux a1b35e56da5adadbc117c3cf17b83948ac657f3c0bd79d47bbe0aa70832b5c8e \
    --aarch64-darwin 6314b195321b3acc6826b1c5d66bb9cf9306c8231c6dbb745f51a04c3bcee235
  [ "$status" -ne 0 ]
  echo "$output" | grep -q 'expected exactly one manifest field'
}


# --- GUI self-update Step 2 manifest generator contract ---

@test "updater manifest generator emits darwin-aarch64 static JSON" {
  script=scripts/ci/generate-updater-manifest.py
  [ -f "$script" ]

  tmp="$(mktemp -d)"
  printf '%s\n' 'trusted-signature-content' >"$tmp/update.sig"

  python3 "$script" \
    --tag v0.3.0 \
    --artifact SchneeForge.app.tar.gz \
    --signature-file "$tmp/update.sig" \
    --output "$tmp/latest.json"

  python3 - "$tmp/latest.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    doc = json.load(f)
assert doc["version"] == "0.3.0", doc
assert set(doc["platforms"]) == {"darwin-aarch64"}, doc
entry = doc["platforms"]["darwin-aarch64"]
assert entry["url"] == "https://github.com/Lamy210/nix_setting/releases/download/v0.3.0/SchneeForge.app.tar.gz", entry
assert entry["signature"] == "trusted-signature-content", entry
PY
}

@test "updater manifest generator is deterministic and fails closed on invalid inputs" {
  script=scripts/ci/generate-updater-manifest.py
  tmp="$(mktemp -d)"
  printf '%s\n' 'trusted-signature-content' >"$tmp/update.sig"

  python3 "$script" --tag v0.3.0 --artifact SchneeForge.app.tar.gz --signature-file "$tmp/update.sig" --output "$tmp/one.json"
  python3 "$script" --tag v0.3.0 --artifact SchneeForge.app.tar.gz --signature-file "$tmp/update.sig" --output "$tmp/two.json"
  cmp "$tmp/one.json" "$tmp/two.json"

  for invalid_tag in \
    not-semver \
    v01.2.3 \
    v1.02.3 \
    v1.2.03 \
    v1.2.3-01 \
    v1.2.3-alpha..1 \
    'v1٢.2.3' \
    v1.2.3-; do
    run python3 "$script" --tag "$invalid_tag" --artifact SchneeForge.app.tar.gz --signature-file "$tmp/update.sig" --output "$tmp/latest.json"
    [ "$status" -ne 0 ]
    [ ! -e "$tmp/latest.json" ]
  done

  python3 "$script" --tag v1.2.3-rc.1+build.5 --artifact SchneeForge.app.tar.gz --signature-file "$tmp/update.sig" --output "$tmp/latest.json"
  python3 - "$tmp/latest.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    doc = json.load(f)
assert doc["version"] == "1.2.3-rc.1+build.5", doc
assert "/v1.2.3-rc.1%2Bbuild.5/" in doc["platforms"]["darwin-aarch64"]["url"], doc
PY

  : >"$tmp/empty.sig"
  run python3 "$script" --tag v0.3.0 --artifact SchneeForge.app.tar.gz --signature-file "$tmp/empty.sig" --output "$tmp/latest.json"
  [ "$status" -ne 0 ]
  [ ! -e "$tmp/latest.json" ]

  run python3 "$script" --tag v0.3.0 --artifact 'https://evil.example/update.tar.gz' --signature-file "$tmp/update.sig" --output "$tmp/latest.json"
  [ "$status" -ne 0 ]
  [ ! -e "$tmp/latest.json" ]
}


# --- Release metadata SemVer contract ---

@test "release metadata scripts share fail-closed SemVer validation" {
  tmp="$(mktemp -d)"
  revision=0123456789abcdef0123456789abcdef01234567

  ./scripts/ci/generate-release-metadata.sh \
    v9.9.9-rc.1+build.5 "$revision" "$tmp/valid.json"
  python3 - "$tmp/valid.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    doc = json.load(f)
assert doc["version"] == "9.9.9-rc.1+build.5", doc
assert doc["channel"] == "preview", doc
assert doc["minimum_schneeforge_version"] == doc["version"], doc
PY

  for invalid_tag in \
    9.9.9 \
    v09.9.9 \
    v9.09.9 \
    v9.9.09 \
    v9.9.9-01 \
    v9.9.9-alpha..1 \
    'v9٩.9.9' \
    v9.9.9-; do
    rm -f "$tmp/invalid.json"
    run ./scripts/ci/generate-release-metadata.sh \
      "$invalid_tag" "$revision" "$tmp/invalid.json"
    [ "$status" -ne 0 ]
    [ ! -e "$tmp/invalid.json" ]
  done

  cp "$tmp/valid.json" "$tmp/tampered.json"
  python3 - "$tmp/tampered.json" <<'PY'
import json, sys
path = sys.argv[1]
with open(path, encoding="utf-8") as f:
    doc = json.load(f)
doc["version"] = "09.9.9"
with open(path, "w", encoding="utf-8") as f:
    json.dump(doc, f)
    f.write("\n")
PY
  run python3 scripts/ci/verify_release_metadata.py \
    v9.9.9-rc.1+build.5 "$tmp/tampered.json"
  [ "$status" -ne 0 ]
  echo "$output" | grep -q "invalid SemVer version"
}

# --- GUI self-update Step 2 release activation contract ---

@test "GUI updater release preparation is activation-gated and fail-closed" {
  release=.github/workflows/release.yml
  dmg=scripts/ci/build-release-macos-dmg.sh
  generator=scripts/ci/generate-updater-manifest.py

  [ -f "$generator" ]

  grep -q 'SCHNEEFORGE_UPDATER_ACTIVATED' "$release"
  grep -q 'SCHNEEFORGE_UPDATER_PUBKEY' "$release"
  grep -q 'TAURI_SIGNING_PRIVATE_KEY' "$release"
  # Production private key must be scoped to the Tauri build step rather than
  # the whole build-dmg job (checkout/cache/diagnostics must not receive it).
  grep -qE '^          TAURI_SIGNING_PRIVATE_KEY:' "$release"
  grep -qE '^          TAURI_SIGNING_PRIVATE_KEY_PASSWORD:' "$release"
  run grep -qE '^      TAURI_SIGNING_PRIVATE_KEY:' "$release"
  [ "$status" -ne 0 ]
  run grep -qE '^      TAURI_SIGNING_PRIVATE_KEY_PASSWORD:' "$release"
  [ "$status" -ne 0 ]
  grep -q 'generate-updater-manifest.py' "$release"
  grep -q 'schneeforge-updater' "$release"
  grep -q 'latest.json' "$release"

  grep -q 'SCHNEEFORGE_UPDATER_ACTIVATED' "$dmg"
  grep -q 'createUpdaterArtifacts' "$dmg"
  grep -q 'TAURI_SIGNING_PRIVATE_KEY' "$dmg"
  grep -q 'SCHNEEFORGE_UPDATER_PUBKEY' "$dmg"

  # PR required CI must stay secret-free.
  run grep -q 'TAURI_SIGNING_PRIVATE_KEY' .github/workflows/check.yml
  [ "$status" -ne 0 ]
}


# --- GUI updater signed-version binding security contract ---

@test "GUI updater requires version-bound signatures and Tauri CLI 2.11.5" {
  dmg=scripts/ci/build-release-macos-dmg.sh
  conf=apps/desktop/src-tauri/tauri.conf.json

  grep -q '^TAURI_CLI_VERSION="2.11.5"$' "$dmg"
  grep -q '^TAURI_CLI_SHA256="7734f1d942dbe6e5fea91c1575452f4bf2cc942e6902f2d9d78513baa8527b24"$' "$dmg"

  python3 - "$conf" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as f:
    conf = json.load(f)
updater = conf["plugins"]["updater"]
assert updater["requireSignedVersion"] is True, updater
assert updater["pubkey"] == "", "static config must not ship a placeholder/test trust root"
PY

  lock=apps/desktop/src-tauri/Cargo.lock
  python3 - "$lock" <<'PY'
import re, sys
text = open(sys.argv[1], encoding="utf-8").read()
def version(name):
    m = re.search(r'\[\[package\]\]\nname = "' + re.escape(name) + r'"\nversion = "([^"]+)"', text)
    assert m, name
    return m.group(1)

def at_least_stable(value, minimum):
    without_build = value.split("+", 1)[0]
    release, separator, _prerelease = without_build.partition("-")
    parts = release.split(".")
    assert len(parts) == 3 and all(part.isdigit() for part in parts), value
    numeric = tuple(int(part) for part in parts)
    return numeric > minimum or (numeric == minimum and not separator)

# Guard the comparator itself: lexical string ordering would incorrectly
# accept 2.9.0 as newer than 2.11.5, and the exact floor must reject prereleases.
assert not at_least_stable("2.9.0", (2, 11, 5))
assert not at_least_stable("2.11.5-rc.1", (2, 11, 5))
assert at_least_stable("2.11.5", (2, 11, 5))
assert at_least_stable("2.12.0", (2, 11, 5))

tauri = version("tauri")
updater = version("tauri-plugin-updater")
assert at_least_stable(tauri, (2, 11, 5)), tauri
assert at_least_stable(updater, (2, 12, 0)), updater
PY
}
