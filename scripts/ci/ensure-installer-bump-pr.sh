#!/usr/bin/env bash
set -euo pipefail

# Run once in read-only preflight mode BEFORE branch preparation/push, then
# again after preparation to catch intervening PR review decisions.
preflight=false
if [ "$#" -eq 3 ] && [ "$1" = --preflight ]; then
  preflight=true
  version=$2
  branch=$3
  body_file=
elif [ "$#" -eq 3 ]; then
  version=$1
  branch=$2
  body_file=$3
else
  echo "usage: ensure-installer-bump-pr.sh [--preflight] VERSION BRANCH [PR_BODY_FILE]" >&2
  exit 2
fi

if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] ||
  [ "$branch" != "bump/nix-installer-$version" ] ||
  { [ "$preflight" = false ] && [ ! -f "$body_file" ]; }; then
  echo "::error::Invalid installer bump PR input." >&2
  exit 2
fi

# REST head filtering includes the owning repository, unlike gh pr list
# --head, which can match a fork-owned PR with the same branch name.
# --paginate prevents silently discarding older closed/merged decisions.
if [[ ! ${GITHUB_REPOSITORY:-} =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
  echo "::error::Missing or invalid GITHUB_REPOSITORY; refusing PR lookup." >&2
  exit 2
fi
owner=$(printf '%s\n' "$GITHUB_REPOSITORY" | cut -d/ -f1)
url="repos/$GITHUB_REPOSITORY/pulls?state=all&head=$owner%3A$branch&base=develop&per_page=100"
if ! matching_prs=$(gh api --paginate "$url" --jq \
  '.[] | "\(.number):\(if .merged_at != null then "MERGED" else (.state | ascii_upcase) end)"'); then
  echo "::error::Cannot list existing installer bump PRs; refusing to create a possible duplicate." >&2
  exit 1
fi

if [ -n "$matching_prs" ]; then
  # Do not guess when a branch has multiple historical pull requests.
  mapfile -t pr_matches <<<"$matching_prs"
  if [ "${#pr_matches[@]}" -ne 1 ]; then
    echo "::error::Multiple installer bump PRs found for $branch; review manually." >&2
    exit 1
  fi
  IFS=: read -r number state <<<"${pr_matches[0]}"
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
elif [ "$preflight" = true ]; then
  echo "No existing installer bump PR for $branch; branch preparation may proceed."
else
  gh pr create \
    --base develop \
    --head "$branch" \
    --title "bump(nix-installer): $version" \
    --body-file "$body_file"
fi
