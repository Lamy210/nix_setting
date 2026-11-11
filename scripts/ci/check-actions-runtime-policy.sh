#!/usr/bin/env bash
# Fail CI if Node.js 20 JavaScript actions or floating Ubuntu runners return.
set -euo pipefail

shopt -s nullglob
workflows=(.github/workflows/*.yml .github/workflows/*.yaml)
if (( ${#workflows[@]} == 0 )); then
  echo 'ERROR: no GitHub Actions workflows found' >&2
  exit 1
fi

for workflow in "${workflows[@]}"; do
  if grep -nF 'ubuntu-latest' "$workflow"; then
    echo "ERROR: floating Ubuntu runner in $workflow" >&2
    exit 1
  fi
  while IFS= read -r line; do
    if [[ "$line" =~ uses:[[:space:]]+([^[:space:]#]+) ]]; then
      ref="${BASH_REMATCH[1]}"
    else
      echo "ERROR: cannot parse action reference: $line" >&2
      exit 1
    fi
    case "$ref" in
      actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1|\
      actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9|\
      actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a|\
      actions/download-artifact@37930b1c2abaa49bbe596cd826c3c89aef350131|\
      softprops/action-gh-release@8a0cd651e5b2365bd640854a3fa5dfc70e4875f3)
        ;;
      *)
        echo "ERROR: unapproved action version or Node.js runtime in $workflow: $ref" >&2
        exit 1
        ;;
    esac
  done < <(grep -E '^[[:space:]]*(-[[:space:]]+)?uses:[[:space:]]+(actions/(checkout|upload-artifact|download-artifact)|softprops/action-gh-release)@' "$workflow" || true)
done

echo 'PASS: pinned Node.js 24 actions and stable Ubuntu images'
