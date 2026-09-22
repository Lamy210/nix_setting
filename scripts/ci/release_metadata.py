#!/usr/bin/env python3
"""Release Metadata (v2 §27) の生成。

TAG と SOURCE_SHA、repo の schneeforge.toml から
schneeforge-release.json を生成する。toml 読み取りに外部依存
(toml/jq) を持たせないため、python3 標準機能のみで必要な項目を
抽出する (schema / [systems] の有効化された key 一覧)。
"""

import json
import re
import sys

from release_semver import channel_for_version, normalize_release_tag

SCHEMA = 1


def parse_manifest(path: str) -> dict:
    """schneeforge.toml から schema と enabled systems を抽出する。

    汎用 toml parser ではなく、本 repo の manifest が取り得る形
    (schema = <int> / [systems] 直下の <name> = <bool>) のみを解釈する。
    """
    schema = None
    systems = []
    in_systems = False
    with open(path, encoding="utf-8") as f:
        for line in f:
            stripped = line.split("#", 1)[0].strip()
            if not stripped:
                continue
            if stripped == "[systems]":
                in_systems = True
                continue
            if stripped.startswith("[") and stripped.endswith("]"):
                in_systems = False
                continue
            if in_systems:
                m = re.match(r"^([A-Za-z0-9_.-]+)\s*=\s*(true|false)\s*$", stripped)
                if not m:
                    raise SystemExit(f"error: unparseable [systems] entry: {stripped!r}")
                if m.group(2) == "true":
                    systems.append(m.group(1))
            else:
                m = re.match(r'^schema\s*=\s*(\d+)\s*$', stripped)
                if m:
                    schema = int(m.group(1))
    if schema is None:
        raise SystemExit("error: schneeforge.toml has no schema")
    if not systems:
        raise SystemExit("error: schneeforge.toml has no enabled [systems]")
    return {"schema": schema, "systems": systems}


def main() -> None:
    if len(sys.argv) != 5:
        raise SystemExit(
            "usage: release_metadata.py <TAG> <SOURCE_SHA> <MANIFEST> <OUT_FILE>"
        )
    tag, source_sha, manifest_path, out_file = sys.argv[1:5]

    try:
        _, version = normalize_release_tag(tag, require_v=True)
    except ValueError as exc:
        raise SystemExit(f"error: {exc}") from exc

    manifest = parse_manifest(manifest_path)
    metadata = {
        "schema": SCHEMA,
        "version": version,
        "channel": channel_for_version(version),
        "source_revision": source_sha,
        # 本 repo では config の提供主体と schneeforge CLI が同一のため
        # release 版数をそのまま最低要件とする
        "minimum_schneeforge_version": version,
        "configuration_schema": manifest["schema"],
        "systems": manifest["systems"],
    }
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(metadata, f, indent=2)
        f.write("\n")


if __name__ == "__main__":
    main()
