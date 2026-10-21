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

if [[ ! $branch =~ ^bump/nix-installer-[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "::error::Refusing unexpected bump branch: $branch" >&2
  exit 2
fi
if [ ! -f "$manifest" ] || [ ! -f "$commit_message" ]; then
  echo '::error::Expected updated manifest and commit message files' >&2
  exit 2
fi

# A workflow_dispatch run may be started on main or a feature branch. Never
# publish a version bump with that branch's unrelated changes as its ancestry.
require_develop_head() {
  local develop_ref develop_sha current_sha
  if ! develop_ref=$(git ls-remote --exit-code --heads origin refs/heads/develop); then
    echo "::error::Cannot verify remote develop HEAD; refusing to create or reuse bump branch." >&2
    return 1
  fi
  develop_sha=$(printf '%s\n' "$develop_ref" | awk '{print $1}')
  current_sha=$(git rev-parse HEAD)
  if [ "$current_sha" != "$develop_sha" ]; then
    echo "::error::Installer bump must start at current origin/develop HEAD, not another workflow_dispatch ref." >&2
    return 1
  fi
}

if remote_head=$(git ls-remote --exit-code --heads origin "refs/heads/$branch"); then
  remote_sha=$(printf '%s\n' "$remote_head" | awk '{print $1}')
  require_develop_head
  # Validate against the immutable SHA returned by ls-remote. Never overwrite
  # a manually recovered PR branch or reset its reviewed commit history.
  git fetch --no-tags --depth=100 origin "refs/heads/$branch"
  fetched_sha=$(git rev-parse FETCH_HEAD)
  if [ "$remote_sha" != "$fetched_sha" ]; then
    echo "::error::Bump branch changed while verifying: $branch" >&2
    exit 1
  fi
  # Reject any unrelated commit content, even when the manifest matches.
  # The scheduled Actions checkout is shallow: fetch enough ancestry to
  # compare the candidate against its common develop ancestor.
  git fetch --no-tags --depth=100 origin develop
  if ! ancestor=$(git merge-base HEAD "$remote_sha"); then
    echo "::error::Cannot verify bump branch ancestry against develop; review manually." >&2
    exit 1
  fi
  changed=$(git diff --name-only "$ancestor" "$remote_sha")
  if [ "$changed" != "$manifest" ]; then
    echo "::error::Unexpected files or changes in existing $branch; only $manifest is allowed" >&2
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
  require_develop_head
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
