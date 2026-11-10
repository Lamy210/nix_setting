#!/usr/bin/env bash
set -euo pipefail

# Boot the disposable systemd container used by managed-nix-e2e.bats.
# Build the systemd image before starting PID 1: package installation at
# container startup can leave boot waiting indefinitely in CI.
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

# A build failure is surfaced immediately. Keep this image solely in the
# disposable runner; no prepared container or privileged host is reused.
if ! docker build --build-arg "BASE_IMAGE=$image" --tag "$image_tag" - <<'DOCKERFILE'
ARG BASE_IMAGE
FROM ${BASE_IMAGE}
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -qq \
    && apt-get install -y -qq systemd systemd-sysv \
    && mkdir -p /opt/repo \
    && rm -rf /var/lib/apt/lists/*
STOPSIGNAL SIGRTMIN+3
CMD ["/sbin/init"]
DOCKERFILE
then
  echo "::error::Could not build the disposable managed Nix systemd image" >&2
  exit 1
fi

if ! docker run -d --privileged --name "$name" "$image_tag" >/dev/null; then
  echo "::error::Could not start managed Nix E2E container $name" >&2
  exit 1
fi

# Continue only when systemd reports a completed boot (degraded is allowed,
# as in the previous gate). Never treat PID 1 merely existing as readiness.
for _ in $(seq 1 60); do
  if docker exec "$name" bash -c 'systemctl is-system-running 2>/dev/null | grep -qE "^(running|degraded)$"'; then
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
