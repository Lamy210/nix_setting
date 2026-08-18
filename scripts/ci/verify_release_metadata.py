#!/usr/bin/env python3
"""生成済み schneeforge-release.json の検証 (生成 script の後段)。

core (release_metadata.rs) と同じ整合規則を CI の生成時に先に課す:
schema==1 / version==TAG / channel が version から導出されるものと一致 /
configuration_schema は現行 manifest schema (1) / systems 非空 /
source_revision が 40 hex。
"""

import json
import re
import sys

EXPECTED_CONFIGURATION_SCHEMA = 1


def channel_for(version: str) -> str:
    return "preview" if re.search(r"-\w+", version) else "stable"


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: verify_release_metadata.py <TAG> <METADATA_FILE>")
    tag, path = sys.argv[1:3]

    with open(path, encoding="utf-8") as f:
        m = json.load(f)

    errors = []
    if m.get("schema") != 1:
        errors.append(f"schema must be 1, got {m.get('schema')!r}")
    version = m.get("version")
    expected_version = tag[1:] if tag.startswith("v") else None
    if expected_version is None:
        errors.append(f"tag must start with 'v': {tag}")
    elif version != expected_version:
        errors.append(f"version {version!r} does not match tag {tag!r}")
    if version and m.get("channel") != channel_for(version):
        errors.append(
            f"channel {m.get('channel')!r} does not match version {version!r}"
        )
    if m.get("configuration_schema") != EXPECTED_CONFIGURATION_SCHEMA:
        errors.append(
            "configuration_schema must be "
            f"{EXPECTED_CONFIGURATION_SCHEMA}, got {m.get('configuration_schema')!r}"
        )
    if not m.get("systems"):
        errors.append("systems must not be empty")
    revision = m.get("source_revision")
    if not isinstance(revision, str) or not re.fullmatch(r"[0-9a-f]{40}", revision):
        errors.append(f"source_revision must be a 40-hex SHA, got {revision!r}")

    if errors:
        for e in errors:
            print(f"error: {e}", file=sys.stderr)
        raise SystemExit(1)

    print(f"release metadata for {tag} verified")


if __name__ == "__main__":
    main()
