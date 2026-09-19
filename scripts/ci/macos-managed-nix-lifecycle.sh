#!/usr/bin/env bash
#
# Manual macOS Apple Silicon acceptance helper for the release-artifact
# Managed Nix CLI lifecycle. This is intentionally narrower than the full
# Finder/install.sh Final Acceptance checklist.
set -euo pipefail

TAG="${1:-}"
ASSET="schneeforge-aarch64-darwin"
RELEASE_BASE="https://github.com/Lamy210/nix_setting/releases/download"
LOG_DIR="${ACCEPTANCE_LOG_DIR:-${RUNNER_TEMP:-/tmp}/schneeforge-macos-lifecycle-logs}"
WORK_DIR="$(mktemp -d "${RUNNER_TEMP:-/tmp}/schneeforge-macos-lifecycle.XXXXXX")"
ROOT_STAGE_DIR="/private/var/db/schneeforge/acceptance"
ROOT_SF="${ROOT_STAGE_DIR}/schneeforge"
SF="${WORK_DIR}/${ASSET}"
CHECKSUMS="${WORK_DIR}/CHECKSUMS.txt"

fail() {
  echo "[acceptance:error] $*" >&2
  exit 1
}

note() {
  echo "[acceptance] $*"
}

verify_uninstall_state() {
  local phase="$1"
  local diag="$LOG_DIR/${phase}-cleanup.log"
  local bad=0

  {
    echo "phase=$phase"
    echo "nix-path-exists=$([ -e /nix ] && echo yes || echo no)"
    echo "nix-mounted=$(mount | grep -E ' on /nix( |$)' >/dev/null 2>&1 && echo yes || echo no)"
    echo "receipt-exists=$([ -e /nix/receipt.json ] && echo yes || echo no)"
    echo "store-exists=$([ -e /nix/store ] && echo yes || echo no)"
    echo "var-exists=$([ -e /nix/var ] && echo yes || echo no)"
    echo "nix-store-volume=$(diskutil apfs list 2>/dev/null | grep -q 'Nix Store' && echo yes || echo no)"
    echo "build-users:"
    dscl . -list /Users 2>/dev/null | grep '^_nixbld' || true
    echo "daemon:"
    sudo launchctl print system/org.nixos.nix-daemon 2>&1 | head -20 || true
    echo "fstab:"
    cat /etc/fstab 2>/dev/null || true
    echo "synthetic.conf:"
    cat /etc/synthetic.conf 2>/dev/null || true
    echo "nix-dir:"
    ls -la /nix 2>&1 || true
    echo "mount-match:"
    mount | grep -E ' on /nix( |$)' || true
  } | tee "$diag"

  mount | grep -E ' on /nix( |$)' >/dev/null 2>&1 && bad=1
  [ -e /nix/receipt.json ] && bad=1
  [ -e /nix/store ] && bad=1
  [ -e /nix/var ] && bad=1
  if [ -d /nix ] && find /nix -mindepth 1 -maxdepth 1 -print -quit | grep -q .; then
    bad=1
  fi
  dscl . -list /Users 2>/dev/null | grep -q '^_nixbld' && bad=1
  sudo launchctl print system/org.nixos.nix-daemon >/dev/null 2>&1 && bad=1
  grep -Eq '(^|[[:space:]])/nix([[:space:]]|$)' /etc/fstab 2>/dev/null && bad=1
  grep -Eq '^[[:space:]]*nix([[:space:]]|$)' /etc/synthetic.conf 2>/dev/null && bad=1
  diskutil apfs list 2>/dev/null | grep -q 'Nix Store' && bad=1

  [ "$bad" -eq 0 ]
}

run_logged() {
  local name="$1"
  shift
  local log="${LOG_DIR}/${name}.log"
  local rc
  if "$@" >"$log" 2>&1; then
    rc=0
  else
    rc=$?
  fi
  cat "$log"
  return "$rc"
}

sanitize_logs() {
  local file tmp host
  host="$(hostname 2>/dev/null || true)"
  for file in "$LOG_DIR"/*.log; do
    [ -f "$file" ] || continue
    tmp="${file}.tmp"
    sed \
      -e "s|${HOME}|<HOME>|g" \
      -e "s|${WORK_DIR}|<WORK_DIR>|g" \
      -e "s|${host}|<HOST>|g" \
      "$file" >"$tmp" || cp "$file" "$tmp"
    mv "$tmp" "$file"
  done
}

cleanup() {
  local rc=$?
  sanitize_logs || true
  sudo rm -f "$ROOT_SF" 2>/dev/null || true
  sudo rmdir "$ROOT_STAGE_DIR" 2>/dev/null || true
  rm -rf "$WORK_DIR"
  exit "$rc"
}
trap cleanup EXIT

[ "${GITHUB_ACTIONS:-}" = "true" ] ||
  fail "this destructive lifecycle helper may run only inside GitHub Actions"

[ -n "$TAG" ] || fail "usage: $0 <release-tag>"
if ! [[ $TAG =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  fail "invalid release tag: $TAG"
fi

[ "$(uname -s)" = "Darwin" ] || fail "macOS is required"
[ "$(uname -m)" = "arm64" ] || fail "Apple Silicon arm64 runner is required"
[ ! -e /nix ] || fail "fresh-host contract violated: /nix already exists"
if command -v nix >/dev/null 2>&1; then
  fail "fresh-host contract violated: nix is already on PATH"
fi

mkdir -p "$LOG_DIR"
cd "$WORK_DIR"
unset NIX_SETTING_DIR || true

{
  echo "tag=$TAG"
  sw_vers
  uname -m
} | tee "$LOG_DIR/environment.log"

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
"$SF" --version | tee "$LOG_DIR/version.log"
printf 'asset=%s\nsha256=%s\n' "$ASSET" "$actual" >"$LOG_DIR/checksum.log"

note "staging verified CLI for privileged lifecycle operations"
sudo install -d -m 0700 "$ROOT_STAGE_DIR"
sudo install -m 0755 "$SF" "$ROOT_SF"
staged_actual="$(sudo shasum -a 256 "$ROOT_SF" | awk '{print $1}')"
[ "$staged_actual" = "$expected" ] ||
  fail "staged CLI SHA256 mismatch: expected=$expected actual=$staged_actual"

note "installing Managed Nix from release binary embedded manifest"
run_logged install-first sudo "$ROOT_SF" nix install --yes

[ -x /nix/var/nix/profiles/default/bin/nix ] ||
  fail "Nix binary missing after install"
sudo test -r /nix/receipt.json || fail "receipt missing after install"
sudo test -r /nix/schneeforge-managed.json || fail "ownership record missing after install"
sudo grep -q '"installer_version"' /nix/schneeforge-managed.json ||
  fail "ownership record missing installer_version"
sudo grep -Eq '"installer_sha256"[[:space:]]*:[[:space:]]*"[0-9a-f]{64}"' /nix/schneeforge-managed.json ||
  fail "ownership record missing valid installer_sha256"

NIX_BIN="/nix/var/nix/profiles/default/bin/nix"
run_logged store-ping "$NIX_BIN" store ping
run_logged flakes "$NIX_BIN" flake metadata "github:Lamy210/nix_setting/${TAG}" --no-write-lock-file
run_logged nix-doctor "$SF" nix doctor

note "verifying second install fails closed with ExistingNixDetected"
if run_logged install-second sudo "$ROOT_SF" nix install --yes; then
  fail "second install unexpectedly succeeded"
else
  second_rc=$?
fi
grep -q 'ExistingNixDetected' "$LOG_DIR/install-second.log" ||
  fail "second install failed for the wrong reason (exit $second_rc)"

note "uninstalling Managed Nix"
run_logged uninstall-first sudo "$ROOT_SF" nix uninstall
verify_uninstall_state "first" || fail "Nix runtime remnants remain after uninstall"

note "reinstalling Managed Nix"
run_logged reinstall sudo "$ROOT_SF" nix install --yes
[ -x /nix/var/nix/profiles/default/bin/nix ] ||
  fail "Nix binary missing after reinstall"
run_logged reinstall-store-ping /nix/var/nix/profiles/default/bin/nix store ping

note "performing final cleanup uninstall"
run_logged uninstall-final sudo "$ROOT_SF" nix uninstall
verify_uninstall_state "final" || fail "Nix runtime remnants remain after final uninstall"

note "Managed Nix release lifecycle acceptance helper passed for $TAG"
