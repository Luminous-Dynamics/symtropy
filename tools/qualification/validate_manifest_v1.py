#!/usr/bin/env python3
"""Fail-closed validator for Luminous qualification manifests v1.

This control-plane tool parses declarative JSON only. It never executes shell
commands, interprets globs, follows symlinks, or performs network access.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any, NoReturn

SCHEMA_ID = "luminous.qualification-manifest.v1"
MAX_MANIFEST_BYTES = 131_072
MAX_PATH_BYTES = 256
MAX_OWNED_PATHS = 256
MAX_PROFILE_FILES = 64
MAX_PARENT_EQUAL_PATHS = 128
MAX_TRANSIENT_PATHS = 128

PROFILE_ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
GIT_SHA1_RE = re.compile(r"^[0-9a-f]{40}$")
ARTIFACT_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")
PATH_COMPONENT_RE = re.compile(r"^[A-Za-z0-9._@+\-]+$")

ROOT_FIELDS = {
    "schema_id",
    "profile_id",
    "profile_version",
    "exact_parent",
    "required_commit_count",
    "owned_paths",
    "profile_files",
    "parent_equal_paths",
    "qualification_suite_id",
    "toolchain_id",
    "allow_transient_paths",
    "artifact_name",
}
PATH_REF_FIELDS = {"path", "kind"}
PROFILE_FILE_FIELDS = {"path", "sha256", "byte_len"}

SUITE_TOOLCHAINS = {
    "schema-only-v1": {"python-3.12.10"},
    "rust-standalone-protocol-v1": {"rust-1.94.0", "rust-1.96.0"},
    "rust-workspace-package-v1": {"rust-1.94.0", "rust-1.96.0"},
    "nix-build-path-v1": {"nix-repo-pinned"},
}


class ManifestValidationError(ValueError):
    """Manifest is malformed, ambiguous, or outside the v1 contract."""


def _fail(message: str) -> NoReturn:
    raise ManifestValidationError(message)


def _reject_constant(value: str) -> NoReturn:
    _fail(f"non-finite JSON number is forbidden: {value}")


def _object_no_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            _fail(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def load_manifest_bytes(raw: bytes) -> dict[str, Any]:
    if len(raw) > MAX_MANIFEST_BYTES:
        _fail(f"manifest exceeds {MAX_MANIFEST_BYTES} bytes")
    if raw.startswith(b"\xef\xbb\xbf"):
        _fail("UTF-8 BOM is forbidden")
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        _fail(f"manifest is not valid UTF-8: {exc}")

    try:
        value = json.loads(
            text,
            object_pairs_hook=_object_no_duplicates,
            parse_constant=_reject_constant,
        )
    except ManifestValidationError:
        raise
    except json.JSONDecodeError as exc:
        _fail(f"invalid JSON: {exc.msg} at line {exc.lineno} column {exc.colno}")

    if not isinstance(value, dict):
        _fail("manifest root must be a JSON object")
    return value


def load_manifest(path: Path) -> tuple[dict[str, Any], str]:
    try:
        raw = path.read_bytes()
    except OSError as exc:
        _fail(f"cannot read manifest: {exc}")
    return load_manifest_bytes(raw), hashlib.sha256(raw).hexdigest()


def _require_exact_fields(
    obj: dict[str, Any], expected: set[str], where: str
) -> None:
    actual = set(obj)
    missing = sorted(expected - actual)
    extra = sorted(actual - expected)
    if missing:
        _fail(f"{where} missing fields: {', '.join(missing)}")
    if extra:
        _fail(f"{where} has unknown fields: {', '.join(extra)}")


def _require_int(value: Any, field: str, minimum: int, maximum: int) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        _fail(f"{field} must be an integer")
    if not minimum <= value <= maximum:
        _fail(f"{field} must be in [{minimum}, {maximum}]")
    return value


def _validate_repo_path(path: Any, field: str) -> str:
    if not isinstance(path, str):
        _fail(f"{field} must be a string")
    if not path:
        _fail(f"{field} cannot be empty")
    if len(path.encode("utf-8")) > MAX_PATH_BYTES:
        _fail(f"{field} exceeds {MAX_PATH_BYTES} UTF-8 bytes")
    if path.startswith("/") or path.endswith("/"):
        _fail(f"{field} must be a normalized repository-relative path")
    if "\\" in path or "//" in path or "\x00" in path:
        _fail(f"{field} contains forbidden path syntax")

    for component in path.split("/"):
        if component in {"", ".", ".."}:
            _fail(f"{field} contains traversal or empty component")
        if not PATH_COMPONENT_RE.fullmatch(component):
            _fail(
                f"{field} contains a non-canonical path component: {component!r}"
            )
    return path


def _validate_path_ref(value: Any, field: str) -> tuple[str, str]:
    if not isinstance(value, dict):
        _fail(f"{field} must be an object")
    _require_exact_fields(value, PATH_REF_FIELDS, field)
    path = _validate_repo_path(value["path"], f"{field}.path")
    kind = value["kind"]
    if kind not in {"file", "prefix"}:
        _fail(f"{field}.kind must be 'file' or 'prefix'")
    return path, kind


def _spec_covers(
    owner_path: str, owner_kind: str, target_path: str
) -> bool:
    if owner_kind == "file":
        return owner_path == target_path
    return target_path == owner_path or target_path.startswith(owner_path + "/")


def _specs_overlap(
    first_path: str,
    first_kind: str,
    second_path: str,
    second_kind: str,
) -> bool:
    return _spec_covers(first_path, first_kind, second_path) or _spec_covers(
        second_path, second_kind, first_path
    )


def _validate_path_ref_list(
    value: Any,
    field: str,
    *,
    minimum: int,
    maximum: int,
) -> list[tuple[str, str]]:
    if not isinstance(value, list):
        _fail(f"{field} must be an array")
    if not minimum <= len(value) <= maximum:
        _fail(f"{field} length must be in [{minimum}, {maximum}]")

    parsed = [
        _validate_path_ref(item, f"{field}[{index}]")
        for index, item in enumerate(value)
    ]
    if len(set(parsed)) != len(parsed):
        _fail(f"{field} contains duplicate path specifications")
    if parsed != sorted(parsed):
        _fail(f"{field} must be sorted by path then kind")

    for index, (path, kind) in enumerate(parsed):
        for other_path, other_kind in parsed[index + 1 :]:
            if _specs_overlap(path, kind, other_path, other_kind):
                _fail(
                    f"{field} contains overlapping path specifications: "
                    f"{path!r} and {other_path!r}"
                )
    return parsed


def _validate_profile_files(value: Any) -> list[dict[str, Any]]:
    if not isinstance(value, list):
        _fail("profile_files must be an array")
    if len(value) > MAX_PROFILE_FILES:
        _fail(f"profile_files exceeds {MAX_PROFILE_FILES} entries")

    parsed: list[dict[str, Any]] = []
    seen: set[str] = set()
    for index, item in enumerate(value):
        field = f"profile_files[{index}]"
        if not isinstance(item, dict):
            _fail(f"{field} must be an object")
        _require_exact_fields(item, PROFILE_FILE_FIELDS, field)

        path = _validate_repo_path(item["path"], f"{field}.path")
        digest = item["sha256"]
        if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
            _fail(
                f"{field}.sha256 must be 64 lowercase hexadecimal characters"
            )
        byte_len = _require_int(
            item["byte_len"],
            f"{field}.byte_len",
            1,
            1_073_741_824,
        )
        if path in seen:
            _fail(f"profile_files contains duplicate path: {path}")
        seen.add(path)
        parsed.append(
            {"path": path, "sha256": digest, "byte_len": byte_len}
        )

    paths = [item["path"] for item in parsed]
    if paths != sorted(paths):
        _fail("profile_files must be sorted by path")
    return parsed


def validate_manifest(value: dict[str, Any]) -> dict[str, Any]:
    _require_exact_fields(value, ROOT_FIELDS, "manifest")

    if value["schema_id"] != SCHEMA_ID:
        _fail(f"schema_id must equal {SCHEMA_ID!r}")

    profile_id = value["profile_id"]
    if (
        not isinstance(profile_id, str)
        or not PROFILE_ID_RE.fullmatch(profile_id)
    ):
        _fail("profile_id is not canonical")

    _require_int(
        value["profile_version"], "profile_version", 1, 4_294_967_295
    )

    exact_parent = value["exact_parent"]
    if (
        not isinstance(exact_parent, str)
        or not GIT_SHA1_RE.fullmatch(exact_parent)
    ):
        _fail(
            "exact_parent must be exactly 40 lowercase hexadecimal characters"
        )

    _require_int(
        value["required_commit_count"], "required_commit_count", 1, 64
    )

    owned = _validate_path_ref_list(
        value["owned_paths"],
        "owned_paths",
        minimum=1,
        maximum=MAX_OWNED_PATHS,
    )
    profiles = _validate_profile_files(value["profile_files"])
    parent_equal = _validate_path_ref_list(
        value["parent_equal_paths"],
        "parent_equal_paths",
        minimum=0,
        maximum=MAX_PARENT_EQUAL_PATHS,
    )
    transient = _validate_path_ref_list(
        value["allow_transient_paths"],
        "allow_transient_paths",
        minimum=0,
        maximum=MAX_TRANSIENT_PATHS,
    )

    suite = value["qualification_suite_id"]
    if not isinstance(suite, str) or suite not in SUITE_TOOLCHAINS:
        _fail("qualification_suite_id is not a reviewed v1 suite")

    toolchain = value["toolchain_id"]
    if (
        not isinstance(toolchain, str)
        or toolchain not in SUITE_TOOLCHAINS[suite]
    ):
        allowed = ", ".join(sorted(SUITE_TOOLCHAINS[suite]))
        _fail(
            f"toolchain_id is incompatible with {suite}; allowed: {allowed}"
        )

    artifact = value["artifact_name"]
    if (
        not isinstance(artifact, str)
        or not ARTIFACT_RE.fullmatch(artifact)
    ):
        _fail("artifact_name is not canonical")

    for owned_path, owned_kind in owned:
        for parent_path, parent_kind in parent_equal:
            if _specs_overlap(
                owned_path, owned_kind, parent_path, parent_kind
            ):
                _fail(
                    "owned_paths and parent_equal_paths overlap: "
                    f"{owned_path!r} and {parent_path!r}"
                )

    protected_specs = (
        [(item["path"], "file") for item in profiles]
        + parent_equal
        + owned
    )
    for transient_path, transient_kind in transient:
        for protected_path, protected_kind in protected_specs:
            if _specs_overlap(
                transient_path,
                transient_kind,
                protected_path,
                protected_kind,
            ):
                _fail(
                    "allow_transient_paths overlaps protected content: "
                    f"{transient_path!r} and {protected_path!r}"
                )

    return value


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args(argv)

    try:
        value, file_sha256 = load_manifest(args.manifest)
        validate_manifest(value)
    except ManifestValidationError as exc:
        print(
            f"QUALIFICATION_MANIFEST_V1 FAIL: {exc}",
            file=sys.stderr,
        )
        return 2

    print("QUALIFICATION_MANIFEST_V1 PASS")
    print(f"profile_id={value['profile_id']}")
    print(f"profile_version={value['profile_version']}")
    print(f"manifest_file_sha256={file_sha256}")
    print(f"qualification_suite_id={value['qualification_suite_id']}")
    print(f"toolchain_id={value['toolchain_id']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
