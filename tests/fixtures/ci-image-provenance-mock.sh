#!/usr/bin/env bash
set -euo pipefail

# Offline fault injection of the pinned GHCR path. No real credentials or Docker.
subject=${1:?path to image preparation script required}
mode=${2:?success, attestation, pull or runtime required}
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
mkdir "$work/bin"
export MOCK_LOG="$work/calls" MOCK_MODE="$mode"
export MOCK_IMAGE="ghcr.io/lamy210/nix-setting-managed-nix-e2e@sha256:1111111111111111111111111111111111111111111111111111111111111111"
: >"$MOCK_LOG"
cat >"$work/bin/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' verify >> "$MOCK_LOG"
[[ "$*" == "attestation verify oci://$MOCK_IMAGE --repo Lamy210/nix_setting --signer-workflow Lamy210/nix_setting/.github/workflows/managed-nix-e2e-image.yml --source-ref refs/heads/develop --deny-self-hosted-runners" ]] || exit 98
[[ "$MOCK_MODE" != attestation ]] || exit 41
SH
cat >"$work/bin/docker" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$1" >> "$MOCK_LOG"
image=schneeforge-managed-nix-e2e-systemd:ci
case "$1" in
  pull)
    [[ "$#" == 2 && "$2" == "$MOCK_IMAGE" ]] || exit 98
    [[ "$MOCK_MODE" != pull ]] || exit 42
    ;;
  tag)
    [[ "$#" == 3 && "$2" == "$MOCK_IMAGE" && "$3" == "$image" ]] || exit 98
    ;;
  run)
    [[ "$#" == 6 && "$2" == --rm && "$3" == "$image" && "$4" == /bin/bash && "$5" == -euc ]] || exit 98
    [[ "$6" == *'test ! -e /nix'* && "$6" == *'systemd systemd-sysv'* ]] || exit 98
    [[ "$MOCK_MODE" != runtime ]] || exit 43
    ;;
  *) exit 98 ;;
esac
SH
cat >"$work/bin/curl" <<'SH'
#!/usr/bin/env bash
printf '%s\n' unexpected-network >> "$MOCK_LOG"
exit 99
SH
chmod +x "$work/bin/gh" "$work/bin/docker" "$work/bin/curl"
status=0
env -u GH_TOKEN -u GITHUB_TOKEN -u GITHUB_ACTOR -u GH_ENTERPRISE_TOKEN -u GITHUB_ENTERPRISE_TOKEN \
  PATH="$work/bin:$PATH" GITHUB_ACTIONS=true SCHNEEFORGE_CI_RUNNER_ENVIRONMENT=github-hosted \
  SCHNEEFORGE_E2E_SYSTEMD_IMAGE="$MOCK_IMAGE" bash "$subject" >"$work/output" 2>&1 || status=$?
case "$mode" in
success)
  expected_status=0
  expected_calls=$'verify\npull\ntag\nrun'
  ;;
attestation)
  expected_status=41
  expected_calls=verify
  ;;
pull)
  expected_status=42
  expected_calls=$'verify\npull'
  ;;
runtime)
  expected_status=43
  expected_calls=$'verify\npull\ntag\nrun'
  ;;
*) exit 2 ;;
esac
if [[ $status != "$expected_status" || "$(cat "$MOCK_LOG")" != "$expected_calls" ]]; then
  printf 'unexpected status/order: %s\n' "$status" >&2
  cat "$MOCK_LOG" "$work/output" >&2
  exit 1
fi
if [[ $mode == success ]]; then
  grep -Fq 'PASS: attested and digest-pinned GHCR systemd E2E image ready' "$work/output"
else
  if grep -Fq 'PASS:' "$work/output"; then
    echo 'Unexpected success marker after verification failure' >&2
    exit 1
  fi
fi
printf 'PASS: mock GHCR %s\n' "$mode"
