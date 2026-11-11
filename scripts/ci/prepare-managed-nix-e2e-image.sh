#!/usr/bin/env bash
set -euo pipefail

# Build an isolated systemd test image from Canonical's signed Ubuntu minimal
# cloud rootfs. Never install packages or Nix on the host runner.
# The release serial is fixed, not the rolling /release symlink.
base_url="https://cloud-images.ubuntu.com/minimal/releases/noble/release-20261027"
archive="ubuntu-24.04-minimal-cloudimg-amd64-root.tar.xz"
manifest="ubuntu-24.04-minimal-cloudimg-amd64-root.manifest"
fingerprint="D2EB44626FDDC30B513D5BB71A5D6C4C7DB87C81"
image="schneeforge-managed-nix-e2e-systemd:ci"

if [[ ${GITHUB_ACTIONS:-} == true && ${SCHNEEFORGE_CI_RUNNER_ENVIRONMENT:-} != github-hosted ]]; then
  echo "ERROR: privileged Managed Nix E2E is restricted to disposable GitHub-hosted runners" >&2
  exit 1
fi

# An approved GHCR image can be consumed by immutable digest only. Verify
# GitHub OIDC provenance from this repository's protected manual build.
# A tag, arbitrary repository, missing attestation or wrong signing workflow
# MUST NOT fall back silently to an unverified container.
pinned_image="${SCHNEEFORGE_E2E_SYSTEMD_IMAGE:-}"
if [[ -n $pinned_image ]]; then
  if [[ ! $pinned_image =~ ^ghcr\.io/lamy210/nix-setting-managed-nix-e2e@sha256:[a-f0-9]{64}$ ]]; then
    echo "ERROR: GHCR test image must be the approved repository pinned by SHA256 digest" >&2
    exit 1
  fi
  command -v gh >/dev/null 2>&1 || {
    echo "ERROR: GitHub CLI required to verify container provenance" >&2
    exit 1
  }
  gh attestation verify "oci://$pinned_image" \
    --repo Lamy210/nix_setting \
    --signer-workflow Lamy210/nix_setting/.github/workflows/managed-nix-e2e-image.yml \
    --source-ref refs/heads/develop \
    --deny-self-hosted-runners
  docker pull "$pinned_image"
  docker tag "$pinned_image" "$image"
  docker run --rm "$image" /bin/bash -euc '
    . /etc/os-release
    test "$ID" = ubuntu
    test "$VERSION_ID" = 24.04
    test -x /sbin/init
    dpkg-query -W -f="${Status}\n" systemd systemd-sysv | grep -c "^install ok installed$" | grep -qx 2
    test ! -e /nix
    command -v systemctl >/dev/null
  '
  echo "PASS: attested and digest-pinned GHCR systemd E2E image ready"
  exit 0
fi

for command in curl gpg gpgv sha256sum docker awk grep; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "ERROR: missing required verification tool: $command" >&2
    exit 1
  fi
done

work="$(mktemp -d)"
trap 'rm -rf -- "$work"' EXIT

download() {
  local filename=$1
  curl --proto '=https' --tlsv1.2 --fail --show-error --location \
    --retry 3 --retry-all-errors --connect-timeout 15 --max-time 900 \
    "$base_url/$filename" --output "$work/$filename"
}

download SHA256SUMS
download SHA256SUMS.gpg
download "$manifest"

# Ubuntu documents the UEC automatic cloud-image signer. Trust only that exact
# fingerprint, even if the supplied keyring has several keys.
keyring="/usr/share/keyrings/ubuntu-cloudimage-keyring.gpg"
if [[ ! -r $keyring ]]; then
  echo "ERROR: Ubuntu cloud-image verification keyring unavailable: $keyring" >&2
  exit 1
fi
gpgv --status-fd 1 --keyring "$keyring" "$work/SHA256SUMS.gpg" "$work/SHA256SUMS" \
  >"$work/signature-status.log"
if ! grep -Fq "[GNUPG:] VALIDSIG $fingerprint " "$work/signature-status.log"; then
  echo "ERROR: Ubuntu cloud-image manifest signed by unexpected key" >&2
  exit 1
fi

verify_checksum() {
  local name=$1 expected
  expected="$(awk -v name="$name" '($2 == name || $2 == "*" name) { print $1 }' "$work/SHA256SUMS")"
  if [[ ! $expected =~ ^[[:xdigit:]]{64}$ ]]; then
    echo "ERROR: missing or ambiguous signed SHA256 for $name" >&2
    exit 1
  fi
  printf '%s  %s\n' "$expected" "$name" | (cd "$work" && sha256sum --check --status -)
  echo "Verified Canonical signed SHA256 for $name: $expected"
}

verify_checksum "$manifest"
for package in systemd systemd-sysv; do
  if ! grep -Eq "^${package}[[:space:]]" "$work/$manifest"; then
    echo "ERROR: official rootfs manifest lacks $package" >&2
    exit 1
  fi
done

download "$archive"
verify_checksum "$archive"

# docker import does not invoke apt-get; the source rootfs already contains
# systemd, curl, Bash and Ubuntu's package database.
docker import "$work/$archive" "$image" >/dev/null

# Verify identity and prerequisites before any privileged container is run.
docker run --rm "$image" /bin/bash -euc '
  . /etc/os-release
  test "$ID" = ubuntu
  test "$VERSION_ID" = 24.04
  test -x /sbin/init
  dpkg-query -W -f="${Status}\n" systemd systemd-sysv | grep -c "^install ok installed$" | grep -qx 2
  test ! -e /nix
  command -v systemctl >/dev/null
'
docker image inspect --format '{{.Id}}' "$image"
echo "PASS: Canonical signed Ubuntu 24.04 systemd image prepared without APT"
