#!/usr/bin/env bash
# Audit every workflow in this checkout, including newly added canary jobs.
# Pin to reviewed Node.js 24 action SHAs; do not silently ignore new files.
set -euo pipefail

shopt -s nullglob
workflows=(.github/workflows/*.yml .github/workflows/*.yaml)
if (("${#workflows[@]}" == 0)); then
  echo "ERROR: no workflow files found" >&2
  exit 1
fi

approved_refs=(
  actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
  actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9
  actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
  actions/download-artifact@37930b1c2abaa49bbe596cd826c3c89aef350131
  actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6
  softprops/action-gh-release@8a0cd651e5b2365bd640854a3fa5dfc70e4875f3
)

for workflow in "${workflows[@]}"; do
  if [[ ! -f $workflow ]]; then
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
  done < <(grep -E '^[[:space:]]*(-[[:space:]]+)?uses:[[:space:]]+(actions/(checkout|upload-artifact|download-artifact|attest)|softprops/action-gh-release)@' "$workflow" || true)
done

printf 'PASS: %s workflows use approved SHA-pinned Actions and no floating ubuntu-latest runner\n' "${#workflows[@]}"
