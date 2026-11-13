#!/usr/bin/env bash
set -euo pipefail

# Boot the disposable systemd container used by managed-nix-e2e.bats.
# The image must have been prepared by prepare-managed-nix-e2e-image.sh
# from a signed Ubuntu cloud rootfs, without any runtime APT installation.
if [ "$#" -ne 2 ] || [ -z "$1" ] || [ -z "$2" ]; then
  echo "usage: start-managed-nix-e2e-container.sh CONTAINER_NAME UBUNTU_IMAGE" >&2
  exit 2
fi
name=$1
image=$2
image_tag="schneeforge-managed-nix-e2e-systemd:ci"

diagnose() {
  echo "::group::Managed Nix E2E systemd diagnostics" >&2
  docker inspect --format 'container={{.Name}} running={{.State.Running}} status={{.State.Status}} exit={{.State.ExitCode}}' "$name" >&2 || true
  docker exec "$name" bash -c 'ps -p 1 -o pid,comm,args; systemctl is-system-running --no-pager || true; systemctl list-jobs --no-pager || true; systemctl --failed --no-pager || true' >&2 || true
  docker logs --tail 160 "$name" >&2 || true
  echo "::endgroup::" >&2
}

docker rm -f "$name" >/dev/null 2>&1 || true

# The image is built only from a Canonical-signed, SHA256-verified rootfs.
# Reject an unexpected image selection rather than silently weakening the OS
# contract. Preparation happens in the same disposable GitHub-hosted runner.
if [ "$image" != ubuntu:24.04 ]; then
  echo "::error::Managed Nix E2E requires Ubuntu 24.04 (requested $image)" >&2
  exit 1
fi
if ! docker image inspect --format '{{.Id}}' "$image_tag" >/dev/null; then
  echo "::error::Verified Canonical systemd image not prepared: $image_tag" >&2
  exit 1
fi

if ! docker run -d --privileged --name "$name" "$image_tag" /sbin/init >/dev/null; then
  echo "::error::Could not start managed Nix E2E container $name" >&2
  exit 1
fi

# Continue only when systemd reports a completed boot (degraded is allowed,
# as in the previous gate). Never treat PID 1 merely existing as readiness.
for _ in $(seq 1 60); do
  if docker exec "$name" bash -c 'systemctl is-system-running 2>/dev/null | grep -qE "^(running|degraded)$"'; then
    # The signed cloud rootfs is imported directly, not built via a Dockerfile.
    # /opt/repo was previously created by Dockerfile RUN, so prepare it here.
    if ! docker exec "$name" mkdir -p /opt/repo; then
      echo "::error::Could not prepare /opt/repo inside Managed Nix E2E container" >&2
      diagnose
      exit 1
    fi
    exit 0
  fi
  if [ "$(docker inspect --format '{{.State.Running}}' "$name" 2>/dev/null)" != true ]; then
    echo "::error::Managed Nix E2E container $name exited before systemd became ready" >&2
    diagnose
    exit 1
  fi
  sleep 5
done

echo "::error::Timed out waiting for systemd in managed Nix E2E container $name" >&2
diagnose
exit 1
