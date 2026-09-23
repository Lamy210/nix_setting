#!/usr/bin/env python3
"""Normalize a nix-installer stable release tag before any network/effectful CI step."""

from __future__ import annotations

import argparse

from release_semver import normalize_release_tag, validate_core_version


def normalize_stable_tag(tag: str) -> str:
    _, version = normalize_release_tag(tag, require_v=False)
    return validate_core_version(version)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("tag")
    args = parser.parse_args()
    try:
        print(normalize_stable_tag(args.tag))
    except ValueError as exc:
        parser.exit(1, f"error: {exc}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
