#!/usr/bin/env bash
set -euo pipefail

# Boot the disposable systemd container used by managed-nix-e2e.bats.
# Do not disguise apt failures as five-minute systemctl timeouts.
if [ "$#" -ne 2 ] || [ -z "$1" ] || [ -z "$2" ]; then
  echo "usage: start-managed-nix-e2e-container.sh CONTAINER_NAME UBUNTU_IMAGE" >&2
  exit 2
fi
name=$1
image=$2

docker rm -f "$name" >/dev/null 2>&1 || true
if ! docker run -d --privileged --name "$name" "$image" \
  bash -c 'set -e; apt-get update -qq; DEBIAN_FRONTEND=noninteractive apt-get install -y -qq systemd systemd-sysv; mkdir -p /opt/repo; exec /sbin/init' >/dev/null; then
  echo "::error::Could not start managed Nix E2E container $name" >&2
  exit 1
fi

# Waiting for systemd is necessary, but never wait out the full timeout once
# the container's PID 1 has already exited. Expose its apt/init diagnostics.
for _ in $(seq 1 60); do
  if docker exec "$name" bash -c 'systemctl is-system-running 2>/dev/null | grep -qE "running|degraded"'; then
    exit 0
  fi
  if [ "$(docker inspect --format '{{.State.Running}}' "$name" 2>/dev/null)" != true ]; then
    echo "::error::Managed Nix E2E container $name exited before systemd became ready" >&2
    docker logs --tail 120 "$name" >&2 || true
    exit 1
  fi
  sleep 5
done

echo "::error::Timed out waiting for systemd in managed Nix E2E container $name" >&2
docker logs --tail 120 "$name" >&2 || true
exit 1
