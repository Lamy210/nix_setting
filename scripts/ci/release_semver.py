"""Shared SemVer validation for release-facing CI scripts."""

from __future__ import annotations

import re

SEMVER_IDENTIFIER = r"(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
SEMVER_RE = re.compile(
    rf"^(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)"
    rf"(?:-(?:{SEMVER_IDENTIFIER})(?:\.(?:{SEMVER_IDENTIFIER}))*)?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$"
)


def validate_version(version: str) -> str:
    if not isinstance(version, str) or not SEMVER_RE.fullmatch(version):
        raise ValueError(f"invalid SemVer version: {version!r}")
    return version



def validate_core_version(version: str) -> str:
    """Validate canonical stable core SemVer: exactly X.Y.Z, no pre-release/build metadata."""
    validate_version(version)
    if "-" in version or "+" in version:
        raise ValueError(f"version must be canonical stable X.Y.Z: {version!r}")
    return version

def normalize_release_tag(tag: str, *, require_v: bool) -> tuple[str, str]:
    if not isinstance(tag, str):
        raise ValueError(f"invalid release tag: {tag!r}")

    if tag.startswith("v"):
        version = tag[1:]
    elif require_v:
        raise ValueError(f"release tag must start with 'v': {tag}")
    else:
        version = tag

    validate_version(version)
    return f"v{version}", version


def channel_for_version(version: str) -> str:
    validate_version(version)
    precedence_version = version.split("+", 1)[0]
    return "preview" if "-" in precedence_version else "stable"
