#!/usr/bin/env bash
# Audit workflows changed by PR #316. Workflows added only to develop after
# this branch was created are outside this PR's tree and need a separate update.
set -euo pipefail

workflows=(
  .github/workflows/check.yml
  .github/workflows/distribution-download-acceptance.yml
  .github/workflows/linux-apply-e2e.yml
  .github/workflows/macos-managed-nix-lifecycle.yml
  .github/workflows/macos-preview-canary.yml
  .github/workflows/managed-nix-bump-acceptance.yml
  .github/workflows/pr-close-cancel.yml
  .github/workflows/release.yml
  .github/workflows/ui-visual-regression.yml
  .github/workflows/update.yml
  .github/workflows/upstream-nix-installer.yml
  .github/workflows/weekly.yml
  .github/workflows/windows-wsl-canary.yml
)
approved_refs=(
  actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
  actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9
  actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
  actions/download-artifact@37930b1c2abaa49bbe596cd826c3c89aef350131
  softprops/action-gh-release@8a0cd651e5b2365bd640854a3fa5dfc70e4875f3
)

for workflow in "${workflows[@]}"; do
  if [[ ! -f "$workflow" ]]; then
    echo "ERROR: expected workflow is missing: $workflow" >&2
    exit 1
  fi
  if grep -nF 'ubuntu-latest' "$workflow"; then
    echo "ERROR: floating Ubuntu runner in $workflow" >&2
    exit 1
  fi
  while IFS= read -r line; do
    if [[ $line =~ uses:[[:space:]]+([^[:space:]#]+) ]]; then
      ref="${BASH_REMATCH[1]}"
    else
      echo "ERROR: cannot parse action reference: $line" >&2
      exit 1
    fi
    approved=false
    for known in "${approved_refs[@]}"; do
      if [[ $ref == "$known" ]]; then
        approved=true
        break
      fi
    done
    if [[ $approved != true ]]; then
      echo "ERROR: unapproved action version or Node.js runtime in $workflow: $ref" >&2
      exit 1
    fi
  done < <(grep -E '^[[:space:]]*(-[[:space:]]+)?uses:[[:space:]]+(actions/(checkout|upload-artifact|download-artifact)|softprops/action-gh-release)@' "$workflow" || true)
done

echo 'PASS: reviewed workflows use SHA-pinned Node.js 24 actions and Ubuntu 24.04'
