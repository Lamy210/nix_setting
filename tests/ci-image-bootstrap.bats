#!/usr/bin/env bats

@test "Canonical Ubuntu rootfs signature and SHA256 must be checked before OCI import" {
  script=scripts/ci/prepare-managed-nix-e2e-image.sh
  [ -f "$script" ]
  grep -Fq 'release-20261027' "$script"
  grep -Fq 'D2EB44626FDDC30B513D5BB71A5D6C4C7DB87C81' "$script"
  grep -Fq 'gpgv --status-fd' "$script"
  grep -Fq 'sha256sum --check --status' "$script"
  grep -Fq 'docker import' "$script"
  grep -Fq 'test ! -e /nix' "$script"
}

@test "publishing is manual, develop-only and attested with pinned Node24 Action" {
  workflow=.github/workflows/managed-nix-e2e-image.yml
  grep -Fq 'workflow_dispatch:' "$workflow"
  grep -Fq "if: github.ref == 'refs/heads/develop'" "$workflow"
  grep -Fq 'actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6' "$workflow"
  grep -Fq 'push-to-registry: true' "$workflow"
  grep -Fq 'packages: write' "$workflow"
  grep -Fq 'id-token: write' "$workflow"
  grep -Fq 'attestations: write' "$workflow"
  grep -Fq 'actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6' scripts/ci/check-actions-runtime-policy.sh
}

@test "preparation rejects a self-hosted runner without downloading or building" {
  run env GITHUB_ACTIONS=true SCHNEEFORGE_CI_RUNNER_ENVIRONMENT=self-hosted \
    bash scripts/ci/prepare-managed-nix-e2e-image.sh
  [ "$status" -ne 0 ]
  [[ "$output" == *"restricted to disposable GitHub-hosted runners"* ]]
}

@test "preparation rejects mutable and arbitrary GHCR image references" {
  run env GITHUB_ACTIONS=false SCHNEEFORGE_E2E_SYSTEMD_IMAGE=ghcr.io/lamy210/nix-setting-managed-nix-e2e:latest \
    bash scripts/ci/prepare-managed-nix-e2e-image.sh
  [ "$status" -ne 0 ]
  [[ "$output" == *"pinned by SHA256 digest"* ]]

  run env GITHUB_ACTIONS=false SCHNEEFORGE_E2E_SYSTEMD_IMAGE=ghcr.io/other/image@sha256:1111111111111111111111111111111111111111111111111111111111111111 \
    bash scripts/ci/prepare-managed-nix-e2e-image.sh
  [ "$status" -ne 0 ]
  [[ "$output" == *"pinned by SHA256 digest"* ]]
}
