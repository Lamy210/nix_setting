#!/usr/bin/env bash
# Release Metadata (v2 §27) の生成と自己検証。
# release.yml (実 tag) と check.yml release-artifact-check (dummy tag) の
# 両方から呼ばれる同一 script (drift 防止の原則)。
#
# usage: generate-release-metadata.sh <TAG> <SOURCE_SHA> [<OUT_FILE>]
#   TAG        release tag (例: v0.2.0-rc.5)
#   SOURCE_SHA tag の commit SHA
#   OUT_FILE   出力先 (default: schneeforge-release.json)
set -euo pipefail

TAG="${1:?usage: $0 <TAG> <SOURCE_SHA> [<OUT_FILE>]}"
SOURCE_SHA="${2:?usage: $0 <TAG> <SOURCE_SHA> [<OUT_FILE>]}"
OUT_FILE="${3:-schneeforge-release.json}"
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

python3 "$REPO_ROOT/scripts/ci/release_metadata.py" \
  "$TAG" "$SOURCE_SHA" "$REPO_ROOT/schneeforge.toml" "$OUT_FILE"

# 生成物の検証 (parse + tag 整合 + channel 整合)。ここで落ちた release は
# asset として出してはならない (1 release = 1 source tree 保証の一部)
python3 "$REPO_ROOT/scripts/ci/verify_release_metadata.py" "$TAG" "$OUT_FILE"

echo "generated $OUT_FILE:"
cat "$OUT_FILE"
