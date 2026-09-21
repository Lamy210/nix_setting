#!/usr/bin/env python3
"""Generate the static Tauri updater manifest for macOS aarch64."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from urllib.parse import quote

REPO_RELEASE_BASE = "https://github.com/Lamy210/nix_setting/releases/download"
TAG_RE = re.compile(r"^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$")
ARTIFACT_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._+-]*$")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    parser.add_argument("--artifact", required=True)
    parser.add_argument("--signature-file", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--notes", default="")
    parser.add_argument("--pub-date")
    return parser.parse_args()


def fail(message: str) -> "None":
    raise ValueError(message)


def normalize_version(tag: str) -> tuple[str, str]:
    version = tag[1:] if tag.startswith("v") else tag
    if not SEMVER_RE.fullmatch(version):
        fail(f"invalid release tag: {tag}")
    return f"v{version}", version


def validate_artifact(name: str) -> str:
    if (
        "/" in name
        or "\\" in name
        or "://" in name
        or name in {".", ".."}
        or not ARTIFACT_RE.fullmatch(name)
    ):
        fail(f"artifact must be a filename, got: {name}")
    return name


def read_signature(path: Path) -> str:
    try:
        signature = path.read_text(encoding="utf-8").strip()
    except OSError as exc:
        fail(f"unable to read signature file: {exc}")
    if not signature:
        fail("signature must not be empty")
    return signature


def build_manifest(args: argparse.Namespace) -> dict[str, object]:
    tag, version = normalize_version(args.tag)
    artifact = validate_artifact(args.artifact)
    signature = read_signature(args.signature_file)
    url = f"{REPO_RELEASE_BASE}/{quote(tag, safe='')}/{quote(artifact, safe='')}"

    document: dict[str, object] = {
        "version": version,
        "platforms": {
            "darwin-aarch64": {
                "url": url,
                "signature": signature,
            }
        },
    }
    if args.notes:
        document["notes"] = args.notes
    if args.pub_date:
        document["pub_date"] = args.pub_date
    return document


def write_atomic(path: Path, document: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp")
    temporary.write_text(
        json.dumps(document, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


def main() -> int:
    args = parse_args()
    try:
        document = build_manifest(args)
        write_atomic(args.output, document)
    except ValueError as exc:
        try:
            args.output.unlink(missing_ok=True)
        except OSError:
            pass
        print(f"error: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
