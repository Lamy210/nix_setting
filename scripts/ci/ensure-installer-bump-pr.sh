#!/usr/bin/env bash
set -euo pipefail

# Preserve review progress on existing nix-installer bump PRs. A failed lookup
# must never be interpreted as "no PR": doing so produces duplicate attempts.
if [ "$#" -ne 3 ]; then
  echo "usage: ensure-installer-bump-pr.sh VERSION BRANCH PR_BODY_FILE" >&2
  exit 2
fi
version=$1
branch=$2
body_file=$3

if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
  [ "$branch" != "bump/nix-installer-$version" ] ||
  [ ! -f "$body_file" ]; then
  echo "::error::Invalid installer bump PR input." >&2
  exit 2
fi

if ! existing_pr=$(gh pr list --head "$branch" --base develop --json number --jq '.[0].number'); then
  echo "::error::Cannot list existing installer bump PRs; refusing to create a possible duplicate." >&2
  exit 1
fi
if [ -n "$existing_pr" ]; then
  echo "PR #$existing_pr already exists; preserving its current body"
else
  gh pr create \
    --base develop \
    --head "$branch" \
    --title "bump(nix-installer): $version" \
    --body-file "$body_file"
fi
