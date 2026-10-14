#!/usr/bin/env python3
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Fail-closed verifier for retained evidence-artifact registry entries."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import stat
import sys
from pathlib import Path
from typing import Any

SCHEMA = "symtropy.evidence.retained-artifact.v1"
HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
ARTIFACT_ID = re.compile(r"^[a-z0-9][a-z0-9._-]{0,127}$")
DECIMAL = re.compile(r"^[0-9]+$")

TOP_KEYS = {"schema", "artifact_id", "artifact", "shape", "availability", "current_replay_context"}
ARTIFACT_KEYS = {
    "logical_name", "media_type", "byte_authority", "sha256",
    "byte_length", "git_blob_sha1_observation",
}
SHAPE_KEYS = {
    "profile", "newline_count", "diff_path_count", "new_path_count",
    "modified_path_count", "deleted_path_count", "renamed_path_count",
    "copied_path_count", "path_set_sha256",
}
AVAILABILITY_KEYS = {
    "status", "github_blob_present", "execution_gate", "known_external_filename",
}
CONTEXT_KEYS = {
    "preflight_pr", "preflight_head", "stage_a_parent_commit", "stage_a_parent_tree",
    "stage_a_run_id", "stage_a_terminal_status", "cargo_lock_repair",
}


class VerificationError(ValueError):
    pass


def fail(message: str) -> "NoReturn":  # type: ignore[name-defined]
    raise VerificationError(message)


def exact_keys(obj: Any, expected: set[str], label: str) -> dict[str, Any]:
    if not isinstance(obj, dict):
        fail(f"{label} must be an object")
    actual = set(obj)
    if actual != expected:
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        fail(f"{label} keys mismatch: missing={missing} extra={extra}")
    return obj


def require_int(value: Any, label: str, *, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        fail(f"{label} must be an integer >= {minimum}")
    return value


def require_str(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        fail(f"{label} must be a non-empty string")
    return value


def require_regular_file(path: Path, label: str) -> int:
    try:
        info = path.lstat()
    except OSError as exc:
        fail(f"cannot inspect {label}: {exc}")
    if stat.S_ISLNK(info.st_mode):
        fail(f"{label} must not be a symlink")
    if not stat.S_ISREG(info.st_mode):
        fail(f"{label} must be a regular file")
    return info.st_size


def load_registry(path: Path) -> dict[str, Any]:
    require_regular_file(path, "registry")
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError as exc:
        fail(f"cannot read registry: {exc}")
    try:
        data = json.loads(raw, parse_constant=lambda token: fail(f"non-standard JSON constant: {token}"))
    except json.JSONDecodeError as exc:
        fail(f"invalid registry JSON: {exc}")
    top = exact_keys(data, TOP_KEYS, "registry")
    if top["schema"] != SCHEMA:
        fail(f"unsupported schema: {top['schema']!r}")
    artifact_id = require_str(top["artifact_id"], "artifact_id")
    if not ARTIFACT_ID.fullmatch(artifact_id):
        fail("artifact_id has invalid syntax")

    artifact = exact_keys(top["artifact"], ARTIFACT_KEYS, "artifact")
    logical_name = require_str(artifact["logical_name"], "artifact.logical_name")
    if "/" in logical_name or "\\" in logical_name or logical_name in {".", ".."}:
        fail("artifact.logical_name must be a basename")
    if artifact["media_type"] != "text/x-diff":
        fail("v1 currently supports only media_type=text/x-diff")
    if artifact["byte_authority"] != "sha256":
        fail("artifact.byte_authority must be sha256")
    if not HEX64.fullmatch(require_str(artifact["sha256"], "artifact.sha256")):
        fail("artifact.sha256 must be lowercase 64-hex")
    require_int(artifact["byte_length"], "artifact.byte_length", minimum=1)
    if not HEX40.fullmatch(require_str(artifact["git_blob_sha1_observation"], "artifact.git_blob_sha1_observation")):
        fail("artifact.git_blob_sha1_observation must be lowercase 40-hex")

    shape = exact_keys(top["shape"], SHAPE_KEYS, "shape")
    if shape["profile"] != "unified-diff-same-path-v1":
        fail("unsupported shape.profile")
    for key in (
        "newline_count", "diff_path_count", "new_path_count", "modified_path_count",
        "deleted_path_count", "renamed_path_count", "copied_path_count",
    ):
        require_int(shape[key], f"shape.{key}")
    if shape["diff_path_count"] != (
        shape["new_path_count"] + shape["modified_path_count"] + shape["deleted_path_count"]
    ):
        fail("shape diff/new/modified/deleted counts are not closed")
    if shape["renamed_path_count"] != 0 or shape["copied_path_count"] != 0:
        fail("v1 profile does not admit rename/copy records")
    if not HEX64.fullmatch(require_str(shape["path_set_sha256"], "shape.path_set_sha256")):
        fail("shape.path_set_sha256 must be lowercase 64-hex")

    availability = exact_keys(top["availability"], AVAILABILITY_KEYS, "availability")
    if availability["status"] not in {"EXTERNAL_RETAINED_BYTES", "GITHUB_RETAINED_BYTES"}:
        fail("unsupported availability.status")
    if not isinstance(availability["github_blob_present"], bool):
        fail("availability.github_blob_present must be boolean")
    if availability["status"] == "EXTERNAL_RETAINED_BYTES" and availability["github_blob_present"]:
        fail("external bytes cannot claim github_blob_present=true")
    if availability["status"] == "GITHUB_RETAINED_BYTES" and not availability["github_blob_present"]:
        fail("GitHub-retained bytes must claim github_blob_present=true")
    if availability["execution_gate"] != "REQUIRE_EXACT_BYTE_MATCH":
        fail("availability.execution_gate must require exact byte match")
    if availability["known_external_filename"] != logical_name:
        fail("availability filename must equal artifact.logical_name")

    context = exact_keys(top["current_replay_context"], CONTEXT_KEYS, "current_replay_context")
    require_int(context["preflight_pr"], "current_replay_context.preflight_pr", minimum=1)
    for key in ("preflight_head", "stage_a_parent_commit", "stage_a_parent_tree"):
        if not HEX40.fullmatch(require_str(context[key], f"current_replay_context.{key}")):
            fail(f"current_replay_context.{key} must be lowercase 40-hex")
    run_id = require_str(context["stage_a_run_id"], "current_replay_context.stage_a_run_id")
    if not DECIMAL.fullmatch(run_id):
        fail("current_replay_context.stage_a_run_id must be ASCII decimal digits")
    if context["stage_a_terminal_status"] != "NOT_NEEDED":
        fail("v1 Universal Matter registry expects Stage A NOT_NEEDED")
    if context["cargo_lock_repair"] != "NOT_AUTHORIZED_OR_REQUIRED":
        fail("cargo_lock_repair claim is inconsistent with NOT_NEEDED")

    return top


def verify_diff_artifact(registry: dict[str, Any], artifact_path: Path) -> None:
    artifact = registry["artifact"]
    shape = registry["shape"]
    if artifact_path.name != artifact["logical_name"]:
        fail(
            "artifact basename mismatch: "
            f"expected={artifact['logical_name']!r} actual={artifact_path.name!r}"
        )
    size = require_regular_file(artifact_path, "artifact")
    if size != artifact["byte_length"]:
        fail(f"byte length mismatch: expected={artifact['byte_length']} actual={size}")

    sha256 = hashlib.sha256()
    git_sha1 = hashlib.sha1()
    git_sha1.update(f"blob {size}\0".encode("ascii"))

    newline_count = 0
    diff_paths: list[bytes] = []
    new_count = deleted_count = renamed_count = copied_count = 0

    try:
        with artifact_path.open("rb") as handle:
            for line in handle:
                sha256.update(line)
                git_sha1.update(line)
                newline_count += line.count(b"\n")
                if line.startswith(b"diff --git "):
                    stripped = line.rstrip(b"\n")
                    match = re.fullmatch(rb"diff --git a/(.+) b/(.+)", stripped)
                    if match is None:
                        fail("diff header is outside unified-diff-same-path-v1")
                    left, right = match.groups()
                    if left != right:
                        fail("diff header changes path; rename/copy not admitted")
                    if not left or b"\x00" in left:
                        fail("invalid diff path")
                    diff_paths.append(right)
                elif line.startswith(b"new file mode "):
                    new_count += 1
                elif line.startswith(b"deleted file mode "):
                    deleted_count += 1
                elif line.startswith(b"rename from "):
                    renamed_count += 1
                elif line.startswith(b"copy from "):
                    copied_count += 1
    except OSError as exc:
        fail(f"cannot read artifact: {exc}")

    actual_sha = sha256.hexdigest()
    if actual_sha != artifact["sha256"]:
        fail(f"sha256 mismatch: expected={artifact['sha256']} actual={actual_sha}")
    actual_git_blob = git_sha1.hexdigest()
    if actual_git_blob != artifact["git_blob_sha1_observation"]:
        fail(
            "git blob SHA-1 observation mismatch: "
            f"expected={artifact['git_blob_sha1_observation']} actual={actual_git_blob}"
        )
    if newline_count != shape["newline_count"]:
        fail(f"newline count mismatch: expected={shape['newline_count']} actual={newline_count}")
    if len(diff_paths) != shape["diff_path_count"]:
        fail(f"diff path count mismatch: expected={shape['diff_path_count']} actual={len(diff_paths)}")
    if len(set(diff_paths)) != len(diff_paths):
        fail("duplicate diff target path")
    if new_count != shape["new_path_count"]:
        fail(f"new path count mismatch: expected={shape['new_path_count']} actual={new_count}")
    if deleted_count != shape["deleted_path_count"]:
        fail(f"deleted path count mismatch: expected={shape['deleted_path_count']} actual={deleted_count}")
    if renamed_count != shape["renamed_path_count"]:
        fail(f"renamed path count mismatch: expected={shape['renamed_path_count']} actual={renamed_count}")
    if copied_count != shape["copied_path_count"]:
        fail(f"copied path count mismatch: expected={shape['copied_path_count']} actual={copied_count}")

    modified_count = len(diff_paths) - new_count - deleted_count
    if modified_count != shape["modified_path_count"]:
        fail(f"modified path count mismatch: expected={shape['modified_path_count']} actual={modified_count}")

    path_set_bytes = b"\n".join(sorted(diff_paths)) + b"\n"
    actual_path_set_sha = hashlib.sha256(path_set_bytes).hexdigest()
    if actual_path_set_sha != shape["path_set_sha256"]:
        fail(
            "path-set sha256 mismatch: "
            f"expected={shape['path_set_sha256']} actual={actual_path_set_sha}"
        )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--registry", required=True, type=Path)
    parser.add_argument("--artifact", type=Path)
    args = parser.parse_args(argv)
    try:
        registry = load_registry(args.registry)
        if args.artifact is None:
            print(
                "REGISTRY_VALID artifact_bytes=NOT_CHECKED "
                f"artifact_id={registry['artifact_id']} sha256={registry['artifact']['sha256']}"
            )
            return 0
        verify_diff_artifact(registry, args.artifact)
        print(
            "ARTIFACT_VERIFIED "
            f"artifact_id={registry['artifact_id']} sha256={registry['artifact']['sha256']}"
        )
        return 0
    except VerificationError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
