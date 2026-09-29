#!/usr/bin/env python3
"""QUAL-001A2 additive hardening for qualification manifest v1.

This layer composes the already-qualified v1 validator and adds two
cross-field/path invariants before QUAL-001B consumes the manifest language:

1. repository control state named by any case-folded `.git` path component is
   unrepresentable on every manifest path surface;
2. every frozen profile file belongs to exactly one explicit lifecycle surface:
   owned by the current theorem or inherited parent-equal.

It remains declarative control-plane validation only: no shell/process
execution, glob expansion, symlink traversal, filesystem mutation, or network
access.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any, NoReturn

QUALIFICATION_DIR = Path(__file__).resolve().parent
if str(QUALIFICATION_DIR) not in sys.path:
    sys.path.insert(0, str(QUALIFICATION_DIR))

import validate_manifest_v1 as base  # noqa: E402


def _fail(message: str) -> NoReturn:
    raise base.ManifestValidationError(message)


def _reject_git_control_path(path: str, field: str) -> None:
    for component in path.split("/"):
        if component.casefold() == ".git":
            _fail(f"{field} names reserved Git control path component: {component!r}")


def _spec_covers(spec: dict[str, Any], target_path: str) -> bool:
    path = spec["path"]
    kind = spec["kind"]
    if kind == "file":
        return path == target_path
    return target_path == path or target_path.startswith(path + "/")


def _validate_git_control_exclusions(value: dict[str, Any]) -> None:
    for field in ("owned_paths", "parent_equal_paths", "allow_transient_paths"):
        for index, spec in enumerate(value[field]):
            _reject_git_control_path(spec["path"], f"{field}[{index}].path")

    for index, profile in enumerate(value["profile_files"]):
        _reject_git_control_path(profile["path"], f"profile_files[{index}].path")


def _validate_profile_lifecycle(value: dict[str, Any]) -> None:
    owned = value["owned_paths"]
    parent_equal = value["parent_equal_paths"]

    for index, profile in enumerate(value["profile_files"]):
        path = profile["path"]
        covered_by_owned = any(_spec_covers(spec, path) for spec in owned)
        covered_by_parent = any(_spec_covers(spec, path) for spec in parent_equal)

        if covered_by_owned == covered_by_parent:
            _fail(
                f"profile_files[{index}].path must be covered by exactly one "
                "lifecycle surface (owned_paths XOR parent_equal_paths): "
                f"{path!r}"
            )


def validate_manifest(value: dict[str, Any]) -> dict[str, Any]:
    """Validate base manifest v1 plus the QUAL-001A2 hardening ratchet."""

    base.validate_manifest(value)
    _validate_git_control_exclusions(value)
    _validate_profile_lifecycle(value)
    return value


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args(argv)

    try:
        value, file_sha256 = base.load_manifest(args.manifest)
        validate_manifest(value)
    except base.ManifestValidationError as exc:
        print(f"QUALIFICATION_MANIFEST_V1_A2 FAIL: {exc}", file=sys.stderr)
        return 2

    print("QUALIFICATION_MANIFEST_V1_A2 PASS")
    print(f"profile_id={value['profile_id']}")
    print(f"profile_version={value['profile_version']}")
    print(f"manifest_file_sha256={file_sha256}")
    print(f"qualification_suite_id={value['qualification_suite_id']}")
    print(f"toolchain_id={value['toolchain_id']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
