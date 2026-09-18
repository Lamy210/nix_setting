#!/usr/bin/env bash
#
# Manual macOS Apple Silicon acceptance helper for the release-artifact
# Managed Nix CLI lifecycle. This is intentionally narrower than the full
# Finder/install.sh Final Acceptance checklist.
set -euo pipefail

TAG="${1:-}"
ASSET="schneeforge-aarch64-darwin"
RELEASE_BASE="https://github.com/Lamy210/nix_setting/releases/download"
ACCEPT_DIR="${RUNNER_TEMP:-/tmp}/schneeforge-acceptance"
WORK_DIR="${ACCEPT_DIR}/work"
ROOT_STAGE_DIR="/private/var/db/schneeforge/acceptance"
ROOT_SF="${ROOT_STAGE_DIR}/schneeforge"
SF="${ACCEPT_DIR}/${ASSET}"
CHECKSUMS="${ACCEPT_DIR}/CHECKSUMS.txt"

fail() {
  echo "[acceptance:error] $*" >&2
  exit 1
}

note() {
  echo "[acceptance] $*"
}

run_logged() {
  local name="$1"
  shift
  local log="${ACCEPT_DIR}/${name}.log"
  "$@" >"$log" 2>&1
  local rc=$?
  cat "$log"
  return "$rc"
}

cleanup_stage() {
  sudo rm -f "$ROOT_SF" 2>/dev/null || true
  sudo rmdir "$ROOT_STAGE_DIR" 2>/dev/null || true
}
trap cleanup_stage EXIT

[ "${GITHUB_ACTIONS:-}" = "true" ] ||
  fail "this destructive lifecycle helper may run only inside GitHub Actions"

[ -n "$TAG" ] || fail "usage: $0 <release-tag>"
if ! [[ "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  fail "invalid release tag: $TAG"
fi

[ "$(uname -s)" = "Darwin" ] || fail "macOS is required"
[ "$(uname -m)" = "arm64" ] || fail "Apple Silicon arm64 runner is required"
[ ! -e /nix ] || fail "fresh-host contract violated: /nix already exists"
if command -v nix >/dev/null 2>&1; then
  fail "fresh-host contract violated: nix is already on PATH"
fi

rm -rf "$ACCEPT_DIR"
mkdir -p "$WORK_DIR"
cd "$WORK_DIR"
unset NIX_SETTING_DIR || true

note "target release: $TAG"
note "downloading release CLI and CHECKSUMS.txt"
curl -fsSL "${RELEASE_BASE}/${TAG}/${ASSET}" -o "$SF"
curl -fsSL "${RELEASE_BASE}/${TAG}/CHECKSUMS.txt" -o "$CHECKSUMS"

expected="$(
  awk -v asset="$ASSET" '
    $2 == asset || $2 ~ ("/" asset "$") { print $1; exit }
  ' "$CHECKSUMS"
)"
[ -n "$expected" ] || fail "CHECKSUMS.txt has no entry for $ASSET"
printf '%s\n' "$expected" | grep -Eq '^[0-9a-f]{64}$' ||
  fail "invalid expected SHA256 for $ASSET"

actual="$(shasum -a 256 "$SF" | awk '{print $1}')"
[ "$actual" = "$expected" ] ||
  fail "release CLI SHA256 mismatch: expected=$expected actual=$actual"
chmod +x "$SF"
"$SF" --version | tee "${ACCEPT_DIR}/version.log"

note "staging verified CLI for privileged lifecycle operations"
sudo install -d -m 0700 "$ROOT_STAGE_DIR"
sudo install -m 0755 "$SF" "$ROOT_SF"
staged_actual="$(sudo shasum -a 256 "$ROOT_SF" | awk '{print $1}')"
[ "$staged_actual" = "$expected" ] ||
  fail "staged CLI SHA256 mismatch: expected=$expected actual=$staged_actual"

note "installing Managed Nix from release binary embedded manifest"
run_logged install sudo "$ROOT_SF" nix install --yes

[ -r /nix/var/nix/profiles/default/bin/nix ] ||
  fail "Nix binary missing after install"
sudo test -r /nix/receipt.json || fail "receipt missing after install"
sudo test -r /nix/schneeforge-managed.json || fail "ownership record missing after install"
sudo grep -q '"installer_version"' /nix/schneeforge-managed.json ||
  fail "ownership record missing installer_version"
sudo grep -Eq '"installer_sha256"[[:space:]]*:[[:space:]]*"[0-9a-f]{64}"'   /nix/schneeforge-managed.json ||
  fail "ownership record missing valid installer_sha256"

NIX_BIN="/nix/var/nix/profiles/default/bin/nix"
run_logged store-ping "$NIX_BIN" store ping
run_logged flakes "$NIX_BIN" flake metadata   "github:Lamy210/nix_setting/${TAG}" --no-write-lock-file
run_logged doctor "$SF" nix doctor

note "verifying second install fails closed"
if run_logged second-install sudo "$ROOT_SF" nix install --yes; then
  fail "second install unexpectedly succeeded"
else
  second_rc=$?
fi
[ "$second_rc" -ne 0 ] || fail "second install must return non-zero"
grep -q 'ExistingNixDetected' "${ACCEPT_DIR}/second-install.log" ||
  fail "second install did not fail with ExistingNixDetected"

note "uninstalling Managed Nix"
run_logged uninstall sudo "$ROOT_SF" nix uninstall
[ ! -e /nix ] || fail "/nix remains after uninstall"

note "reinstalling Managed Nix"
run_logged reinstall sudo "$ROOT_SF" nix install --yes
[ -x /nix/var/nix/profiles/default/bin/nix ] ||
  fail "Nix binary missing after reinstall"
run_logged reinstall-store-ping /nix/var/nix/profiles/default/bin/nix store ping

note "performing final cleanup uninstall"
run_logged final-uninstall sudo "$ROOT_SF" nix uninstall
[ ! -e /nix ] || fail "/nix remains after final uninstall"

note "Managed Nix release lifecycle acceptance helper passed for $TAG"
