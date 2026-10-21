#!/usr/bin/env bash
set -euo pipefail

# Maintain the reviewable update branch without rewriting its history.
# Called after upstream binaries and manifest values have been verified.
if [ "$#" -ne 3 ]; then
  echo 'usage: prepare-upstream-bump-branch.sh MANIFEST BRANCH COMMIT_MESSAGE_FILE' >&2
  exit 2
fi
manifest=$1
branch=$2
commit_message=$3

if [[ ! "$branch" =~ ^bump/nix-installer-[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "::error::Refusing unexpected bump branch: $branch" >&2
  exit 2
fi
if [ ! -f "$manifest" ] || [ ! -f "$commit_message" ]; then
  echo '::error::Expected updated manifest and commit message files' >&2
  exit 2
fi

if remote_head=$(git ls-remote --exit-code --heads origin "refs/heads/$branch"); then
  remote_sha=$(printf '%s\n' "$remote_head" | awk '{print $1}')
  # Validate against the immutable SHA returned by ls-remote. Never overwrite
  # a manually recovered PR branch or reset its reviewed commit history.
  git fetch --no-tags origin "refs/heads/$branch"
  fetched_sha=$(git rev-parse FETCH_HEAD)
  if [ "$remote_sha" != "$fetched_sha" ]; then
    echo "::error::Bump branch changed while verifying: $branch" >&2
    exit 1
  fi
  if ! git show "$remote_sha:$manifest" | cmp -s - "$manifest"; then
    echo "::error::Existing $branch manifest differs from freshly verified version/hashes; refusing branch overwrite. Review manually." >&2
    exit 1
  fi
  echo "Existing $branch exactly matches verified manifest; preserving remote commits and PR review."
else
  rc=$?
  if [ "$rc" -ne 2 ]; then
    echo "::error::Cannot inspect remote branch $branch (git exit $rc); refusing push." >&2
    exit "$rc"
  fi
  git checkout -b "$branch"
  git add -- "$manifest"
  if git diff --cached --quiet -- "$manifest"; then
    echo "::error::No manifest changes to commit for new $branch" >&2
    exit 1
  fi
  git commit -F "$commit_message"
  # Fail on concurrent branch creation; never rewrite review history.
  git push origin "$branch"
fi
