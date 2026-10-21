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

# Include closed and merged PRs: a human may have rejected this exact bump.
# Reopening that release automatically would override the review decision.
if ! matching_prs=$(gh pr list --state all --head "$branch" --base develop \
    --limit 100 --json number,state --jq '.[] | [.number, .state] | @tsv'); then
  echo "::error::Cannot list existing installer bump PRs; refusing to create a possible duplicate." >&2
  exit 1
fi

if [ -n "$matching_prs" ]; then
  # Multiple historical PRs with the same head should be reviewed manually.
  if [[ $matching_prs == *
\n'* ]]; then
    echo "::error::Multiple installer bump PRs found for $branch; review manually." >&2
    exit 1
  fi
  IFS=
\t' read -r number state <<< "$matching_prs"
  if [[ ! $number =~ ^[0-9]+$ ]] ||
    [[ $state != OPEN && $state != CLOSED && $state != MERGED ]]; then
    echo "::error::Unexpected installer bump PR lookup response; refusing create." >&2
    exit 1
  fi
  if [ "$state" != OPEN ]; then
    echo "::error::PR #$number is $state; refusing to recreate a reviewed installer bump." >&2
    exit 1
  fi
  echo "PR #$number already exists; preserving its current body"
else
  gh pr create \
    --base develop \
    --head "$branch" \
    --title "bump(nix-installer): $version" \
    --body-file "$body_file"
fi
