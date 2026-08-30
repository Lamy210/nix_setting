#!/usr/bin/env python3
"""SLSA v0.2 provenance predicate generator for release asset attestation.

SchneeSoftwareDistribution platform 側の検証契約
(packages/application/src/attestations/verify-attestations.ts) に合わせる:
- predicate.builder.id          : release workflow の ref (builder 検証)
- predicate.materials[].digest.sha1 : tag の commit SHA (sourceRevision 検証)
- subject (name / sha256)       : cosign attest-blob が blob から自動設定

usage: slsa_predicate.py <TAG> <SOURCE_SHA> <WORKFLOW_REF_SUFFIX> <OUT_FILE>
  TAG                  release tag (例: v0.2.0)
  SOURCE_SHA           tag の commit SHA (40 hex)
  WORKFLOW_REF_SUFFIX  workflow ref の @ 以降 (例: refs/tags/v0.2.0)
  OUT_FILE             出力先 JSON path
"""

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

REPOSITORY = "Lamy210/nix_setting"
WORKFLOW_PATH = ".github/workflows/release.yml"


def main() -> int:
    if len(sys.argv) != 5:
        sys.stderr.write(f"usage: {sys.argv[0]} <TAG> <SOURCE_SHA> <WORKFLOW_REF_SUFFIX> <OUT_FILE>\n")
        return 2
    tag, source_sha, ref_suffix, out_file = sys.argv[1:5]
    if len(source_sha) != 40 or any(c not in "0123456789abcdef" for c in source_sha.lower()):
        sys.stderr.write(f"invalid source sha: {source_sha}\n")
        return 2

    builder_id = f"https://github.com/{REPOSITORY}/{WORKFLOW_PATH}@{ref_suffix}"
    now = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    predicate = {
        "builder": {"id": builder_id},
        "buildType": f"https://github.com/{REPOSITORY}/release-workflow@v1",
        "invocation": {
            "configSource": {
                "uri": f"git+https://github.com/{REPOSITORY}",
                "digest": {"sha1": source_sha},
                "entryPoint": WORKFLOW_PATH,
            },
            "parameters": {"tag": tag},
            "environment": {"runner": "github-hosted"},
        },
        "metadata": {
            "buildInvocationId": f"{tag}-{source_sha[:12]}",
            "buildStartedOn": now,
            "buildFinishedOn": now,
            "completeness": {"parameters": True, "environment": False, "materials": True},
            "reproducible": False,
        },
        "materials": [
            {"uri": f"git+https://github.com/{REPOSITORY}", "digest": {"sha1": source_sha}}
        ],
    }
    Path(out_file).write_text(json.dumps(predicate, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"generated {out_file}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
