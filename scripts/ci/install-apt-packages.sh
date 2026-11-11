#!/usr/bin/env bash
# Install mandatory CI dependencies. Transient mirror/DNS failure may recover,
# but exhausted retries MUST fail the job (no skipped E2E or hidden failures).
set -euo pipefail

if (($# == 0)); then
  echo "Usage: install-apt-packages.sh <package> [package ...]" >&2
  exit 2
fi

# On the disposable Ubuntu 24.04 x64 runner, the Azure mirror can hang while
# the official Ubuntu archives remain reachable. Use an isolated sources list
# after the first failure rather than modifying the runner's system APT config.
# This retains Ubuntu archive signature verification via the trusted keyring.
fallback_sources=""
if [[ -r /etc/os-release && -r /usr/share/keyrings/ubuntu-archive-keyring.gpg ]]; then
  # shellcheck source=/dev/null
  . /etc/os-release
  if [[ ${ID:-} == ubuntu && ${VERSION_CODENAME:-} == noble ]]; then
    fallback_sources="$(mktemp "${RUNNER_TEMP:-/tmp}/schneeforge-ubuntu-apt.XXXXXX.list")"
    cat > "$fallback_sources" <<'SOURCES'
deb [signed-by=/usr/share/keyrings/ubuntu-archive-keyring.gpg] https://archive.ubuntu.com/ubuntu noble noble-updates noble-backports main restricted universe multiverse
deb [signed-by=/usr/share/keyrings/ubuntu-archive-keyring.gpg] https://security.ubuntu.com/ubuntu noble-security main restricted universe multiverse
SOURCES
    trap 'rm -f -- "$fallback_sources"' EXIT
  fi
fi

apt_opts=(
  -o Acquire::Retries=1
  -o Acquire::http::Timeout=15
  -o Acquire::https::Timeout=15
)

for attempt in 1 2 3; do
  sources=()
  if ((attempt > 1)) && [[ -n $fallback_sources ]]; then
    sources=(
      -o "Dir::Etc::sourcelist=$fallback_sources"
      -o Dir::Etc::sourceparts=-
    )
    echo "APT retry $attempt/3: using signed official Ubuntu archives (temporary sources file)"
  else
    echo "APT dependencies attempt $attempt/3: runner image configured mirrors"
  fi

  if timeout --signal=TERM --kill-after=5s 75s \
    sudo apt-get "${apt_opts[@]}" "${sources[@]}" \
    -o APT::Update::Error-Mode=any update -qq &&
    timeout --signal=TERM --kill-after=5s 135s \
      sudo apt-get "${apt_opts[@]}" "${sources[@]}" \
      install -y -qq -- "$@"; then
    echo "PASS: mandatory APT dependencies installed: $*"
    exit 0
  fi
  if ((attempt == 3)); then
    echo "ERROR: mandatory APT dependencies unavailable after 3 bounded attempts: $*" >&2
    exit 1
  fi
  echo "APT attempt $attempt failed; retrying after backoff (no CI gate bypass)" >&2
  sleep $((attempt * 10))
done
