#!/usr/bin/env python3
"""Normalize a nix-installer stable release tag before any network/effectful CI step."""

from __future__ import annotations

import argparse

from release_semver import channel_for_version, normalize_release_tag


def normalize_stable_tag(tag: str) -> str:
    _, version = normalize_release_tag(tag, require_v=False)
    if channel_for_version(version) != "stable":
        raise ValueError(f"nix-installer bump requires a stable release tag: {tag}")
    return version


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
