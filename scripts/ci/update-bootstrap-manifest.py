#!/usr/bin/env python3
"""Update the pinned Managed Nix version and hashes without rewriting manifest metadata."""

from __future__ import annotations

import argparse
import re
from pathlib import Path

from release_semver import validate_core_version


SHA256_RE = re.compile(r"^[0-9A-Fa-f]{64}$")


def validate_pin_inputs(version: str, hashes: tuple[str, str, str]) -> tuple[str, tuple[str, str, str]]:
    validate_core_version(version)

    normalized: list[str] = []
    for value in hashes:
        if not SHA256_RE.fullmatch(value):
            raise ValueError(f"invalid SHA256 digest: {value!r}")
        normalized.append(value.lower())
    return version, (normalized[0], normalized[1], normalized[2])


def replace_exactly_once(text: str, pattern: str, replacement: str) -> str:
    updated, count = re.subn(pattern, replacement, text)
    if count != 1:
        raise ValueError(f"expected exactly one manifest field for {pattern!r}, found {count}")
    return updated


def update_manifest(
    path: Path,
    *,
    version: str,
    x86_64_linux: str,
    aarch64_linux: str,
    aarch64_darwin: str,
) -> None:
    version, hashes = validate_pin_inputs(
        version, (x86_64_linux, aarch64_linux, aarch64_darwin)
    )
    x86_64_linux, aarch64_linux, aarch64_darwin = hashes
    text = path.read_text(encoding="utf-8")
    replacements = (
        (r'(?m)^version = "[^"]+"$', f'version = "{version}"'),
        (
            r'(?m)^x86_64-linux = "[0-9a-f]{64}"$',
            f'x86_64-linux = "{x86_64_linux}"',
        ),
        (
            r'(?m)^aarch64-linux = "[0-9a-f]{64}"$',
            f'aarch64-linux = "{aarch64_linux}"',
        ),
        (
            r'(?m)^aarch64-darwin = "[0-9a-f]{64}"$',
            f'aarch64-darwin = "{aarch64_darwin}"',
        ),
    )
    for pattern, replacement in replacements:
        text = replace_exactly_once(text, pattern, replacement)
    path.write_text(text, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--x86-64-linux", required=True)
    parser.add_argument("--aarch64-linux", required=True)
    parser.add_argument("--aarch64-darwin", required=True)
    args = parser.parse_args()

    try:
        update_manifest(
            args.manifest,
            version=args.version,
            x86_64_linux=args.x86_64_linux,
            aarch64_linux=args.aarch64_linux,
            aarch64_darwin=args.aarch64_darwin,
        )
    except (OSError, ValueError) as exc:
        parser.exit(1, f"error: {exc}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
