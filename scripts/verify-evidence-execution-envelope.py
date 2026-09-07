#!/usr/bin/env python3
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Verify a retained execution-identity evidence envelope fail-closed.

The verifier accepts either an unpacked evidence directory or the exact retained
ZIP artifact. It verifies the four-file v1 envelope, checksum files, canonical
JSON encoding, semantic/observation cross-bindings, and optional externally
expected provenance such as commit/tree/workflow/run IDs.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import stat
import zipfile
from pathlib import Path

IDENTITY_SCHEMA = "symtropy.evidence.execution-identity.v1"
OBSERVATION_SCHEMA = "symtropy.evidence.execution-observation.v1"
EXPECTED_FILES = (
    "EXECUTION_IDENTITY.json",
    "EXECUTION_IDENTITY.sha256",
    "OBSERVATION.json",
    "OBSERVATION.sha256",
)
HEX64_RE = re.compile(r"^[0-9a-f]{64}$")
OBJECT_ID_RE = re.compile(r"^[0-9a-f]{40}(?:[0-9a-f]{24})?$")
REPOSITORY_RE = re.compile(r"^[a-z0-9_.-]+/[a-z0-9_.-]+$")
BINDING_KEY_RE = re.compile(
    r"^(?:action|environment|input|policy|source|tool|producer|verifier)\.[a-z0-9][a-z0-9_.-]*$"
)
HASH_BINDING_NAMESPACES = {"action", "policy", "source", "producer", "verifier"}
PRINTABLE_ASCII_RE = re.compile(r"^[\x20-\x7e]+$")
IDENTITY_TOP_LEVEL_KEYS = {"schema", "identity", "binding_contract", "bindings"}
IDENTITY_CORE_KEYS = {
    "repository",
    "execution_commit",
    "execution_tree",
    "workflow_path",
    "workflow_blob",
}
OBSERVATION_TOP_LEVEL_KEYS = {
    "schema",
    "execution_identity_sha256",
    "github",
    "runner",
    "host",
    "tools",
    "transport",
}
OBSERVATION_GITHUB_KEYS = {
    "repository",
    "run_id",
    "run_attempt",
    "event_name",
    "workflow_ref",
    "workflow_sha",
}
OBSERVATION_RUNNER_KEYS = {
    "name",
    "os",
    "arch",
    "environment",
    "image_os",
    "image_version",
}
OBSERVATION_HOST_KEYS = {"kernel_release", "os_release_sha256"}
OBSERVATION_TOOL_KEYS = {"git", "python"}
OBSERVATION_TRANSPORT_KEYS = {"upload_artifact_commit"}


class VerificationError(ValueError):
    pass


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _canonical_json_bytes(value: object) -> bytes:
    return (
        json.dumps(
            value,
            ensure_ascii=False,
            allow_nan=False,
            sort_keys=True,
            separators=(",", ":"),
        )
        + "\n"
    ).encode("utf-8")


def _read_directory(path: Path) -> dict[str, bytes]:
    if not path.is_dir():
        raise VerificationError(f"evidence path is not a directory: {path}")
    entries = sorted(child.name for child in path.iterdir())
    if entries != sorted(EXPECTED_FILES):
        raise VerificationError(
            "evidence directory must contain exactly the v1 envelope files; "
            f"found: {entries}"
        )
    files: dict[str, bytes] = {}
    for name in EXPECTED_FILES:
        child = path / name
        if not child.is_file() or child.is_symlink():
            raise VerificationError(f"evidence member must be a regular file: {name}")
        files[name] = child.read_bytes()
    return files


def _read_zip(path: Path) -> dict[str, bytes]:
    if not path.is_file():
        raise VerificationError(f"evidence ZIP does not exist: {path}")
    try:
        archive = zipfile.ZipFile(path)
    except zipfile.BadZipFile as error:
        raise VerificationError("evidence ZIP is invalid") from error

    with archive:
        infos = archive.infolist()
        names = [info.filename for info in infos]
        if len(names) != len(set(names)):
            raise VerificationError("evidence ZIP contains duplicate member names")
        if sorted(names) != sorted(EXPECTED_FILES):
            raise VerificationError(
                "evidence ZIP must contain exactly the four top-level v1 envelope files; "
                f"found: {sorted(names)}"
            )

        files: dict[str, bytes] = {}
        for info in infos:
            if info.is_dir() or "/" in info.filename or "\\" in info.filename:
                raise VerificationError(
                    f"evidence ZIP member must be a top-level regular file: {info.filename!r}"
                )
            unix_mode = (info.external_attr >> 16) & 0xFFFF
            if unix_mode and stat.S_IFMT(unix_mode) not in (0, stat.S_IFREG):
                raise VerificationError(
                    f"evidence ZIP member is not a regular file: {info.filename!r}"
                )
            files[info.filename] = archive.read(info)
        return files


def _parse_checksum(data: bytes, expected_name: str) -> str:
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as error:
        raise VerificationError(f"{expected_name}.sha256 is not ASCII") from error
    match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9_.-]+)\n", text)
    if match is None:
        raise VerificationError(f"invalid checksum-file format for {expected_name}")
    digest, filename = match.groups()
    if filename != expected_name:
        raise VerificationError(
            f"checksum file names {filename!r}, expected {expected_name!r}"
        )
    return digest


def _parse_json(name: str, data: bytes) -> dict[str, object]:
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as error:
        raise VerificationError(f"{name} is not UTF-8") from error
    try:
        value = json.loads(text)
    except json.JSONDecodeError as error:
        raise VerificationError(f"{name} is not valid JSON") from error
    if not isinstance(value, dict):
        raise VerificationError(f"{name} top level must be an object")
    if _canonical_json_bytes(value) != data:
        raise VerificationError(f"{name} is not canonical JSON")
    return value


def _expect_equal(label: str, actual: object, expected: object | None) -> None:
    if expected is not None and actual != expected:
        raise VerificationError(f"{label} mismatch: got {actual!r}, expected {expected!r}")


def _require_exact_keys(label: str, value: dict[str, object], expected: set[str]) -> None:
    actual = set(value)
    if actual != expected:
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        raise VerificationError(
            f"{label} key set mismatch; missing={missing}, extra={extra}"
        )


def _require_string(label: str, value: object, *, allow_empty: bool = False) -> str:
    if not isinstance(value, str):
        raise VerificationError(f"{label} must be a string")
    if not allow_empty and not value:
        raise VerificationError(f"{label} must not be empty")
    return value


def _validate_workflow_path(value: object) -> str:
    path = _require_string("identity workflow_path", value)
    if "\\" in path or path.startswith("/"):
        raise VerificationError("identity workflow_path must be repository-relative POSIX")
    parts = path.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        raise VerificationError("identity workflow_path contains ambiguous path components")
    return path


def _validate_bindings(bindings: dict[str, object]) -> None:
    for key, raw_value in bindings.items():
        if BINDING_KEY_RE.fullmatch(key) is None:
            raise VerificationError(f"invalid binding key: {key!r}")
        value = _require_string(f"binding {key}", raw_value)
        namespace = key.split(".", 1)[0]
        if namespace in HASH_BINDING_NAMESPACES:
            if OBJECT_ID_RE.fullmatch(value) is None:
                raise VerificationError(
                    f"binding {key} must be a 40- or 64-character lowercase hex identity"
                )
        else:
            if PRINTABLE_ASCII_RE.fullmatch(value) is None or value != value.strip():
                raise VerificationError(
                    f"binding {key} must be printable ASCII with no edge whitespace"
                )


def verify_envelope(args: argparse.Namespace) -> dict[str, str]:
    path = args.evidence
    is_zip = path.is_file()
    files = _read_zip(path) if is_zip else _read_directory(path)

    if args.expected_archive_sha256 is not None:
        if not is_zip:
            raise VerificationError("--expected-archive-sha256 requires a ZIP evidence path")
        archive_digest = _sha256(path.read_bytes())
        _expect_equal("archive SHA-256", archive_digest, args.expected_archive_sha256)
    else:
        archive_digest = _sha256(path.read_bytes()) if is_zip else ""

    identity_bytes = files["EXECUTION_IDENTITY.json"]
    observation_bytes = files["OBSERVATION.json"]
    identity_digest = _sha256(identity_bytes)
    observation_digest = _sha256(observation_bytes)

    identity_checksum = _parse_checksum(
        files["EXECUTION_IDENTITY.sha256"], "EXECUTION_IDENTITY.json"
    )
    observation_checksum = _parse_checksum(
        files["OBSERVATION.sha256"], "OBSERVATION.json"
    )
    _expect_equal("identity checksum", identity_digest, identity_checksum)
    _expect_equal("observation checksum", observation_digest, observation_checksum)

    identity = _parse_json("EXECUTION_IDENTITY.json", identity_bytes)
    observation = _parse_json("OBSERVATION.json", observation_bytes)

    _require_exact_keys("identity top level", identity, IDENTITY_TOP_LEVEL_KEYS)
    _require_exact_keys("observation top level", observation, OBSERVATION_TOP_LEVEL_KEYS)
    _expect_equal("identity schema", identity.get("schema"), IDENTITY_SCHEMA)
    _expect_equal("observation schema", observation.get("schema"), OBSERVATION_SCHEMA)
    _expect_equal(
        "observation identity digest",
        observation.get("execution_identity_sha256"),
        identity_digest,
    )

    identity_core = identity.get("identity")
    bindings = identity.get("bindings")
    contract = identity.get("binding_contract")
    if not isinstance(identity_core, dict):
        raise VerificationError("identity.identity must be an object")
    if not isinstance(bindings, dict):
        raise VerificationError("identity.bindings must be an object")
    if not isinstance(contract, dict):
        raise VerificationError("identity.binding_contract must be an object")
    _require_exact_keys("identity.identity", identity_core, IDENTITY_CORE_KEYS)
    _require_exact_keys("identity.binding_contract", contract, {"required"})
    _validate_bindings(bindings)
    required = contract.get("required")
    if not isinstance(required, list) or not all(isinstance(key, str) for key in required):
        raise VerificationError("binding_contract.required must be a string list")
    if required != sorted(set(required)):
        raise VerificationError("binding_contract.required must be sorted and unique")
    missing = [key for key in required if key not in bindings]
    if missing:
        raise VerificationError(
            "required bindings missing from identity: " + ", ".join(missing)
        )

    repository = identity_core.get("repository")
    commit = identity_core.get("execution_commit")
    tree = identity_core.get("execution_tree")
    workflow_blob = identity_core.get("workflow_blob")
    workflow_path = identity_core.get("workflow_path")
    for label, value in (
        ("execution_commit", commit),
        ("execution_tree", tree),
        ("workflow_blob", workflow_blob),
    ):
        if not isinstance(value, str) or OBJECT_ID_RE.fullmatch(value) is None:
            raise VerificationError(f"identity {label} is not a canonical object id")
    if (
        not isinstance(repository, str)
        or repository != repository.lower()
        or REPOSITORY_RE.fullmatch(repository) is None
    ):
        raise VerificationError("identity repository must be normalized lowercase owner/name")
    workflow_path = _validate_workflow_path(workflow_path)

    github = observation.get("github")
    runner = observation.get("runner")
    host = observation.get("host")
    tools = observation.get("tools")
    transport = observation.get("transport")
    for label, value in (
        ("observation.github", github),
        ("observation.runner", runner),
        ("observation.host", host),
        ("observation.tools", tools),
        ("observation.transport", transport),
    ):
        if not isinstance(value, dict):
            raise VerificationError(f"{label} must be an object")

    _require_exact_keys("observation.github", github, OBSERVATION_GITHUB_KEYS)
    _require_exact_keys("observation.runner", runner, OBSERVATION_RUNNER_KEYS)
    _require_exact_keys("observation.host", host, OBSERVATION_HOST_KEYS)
    _require_exact_keys("observation.tools", tools, OBSERVATION_TOOL_KEYS)
    _require_exact_keys("observation.transport", transport, OBSERVATION_TRANSPORT_KEYS)

    _require_string("observation github.run_id", github.get("run_id"))
    _require_string("observation github.run_attempt", github.get("run_attempt"))
    _require_string("observation github.event_name", github.get("event_name"))
    for optional_label, optional_value in (
        ("observation github.workflow_ref", github.get("workflow_ref")),
        ("observation github.workflow_sha", github.get("workflow_sha")),
        ("observation runner.name", runner.get("name")),
        ("observation runner.os", runner.get("os")),
        ("observation runner.arch", runner.get("arch")),
        ("observation runner.environment", runner.get("environment")),
        ("observation runner.image_os", runner.get("image_os")),
        ("observation runner.image_version", runner.get("image_version")),
    ):
        if optional_value is not None and not isinstance(optional_value, str):
            raise VerificationError(f"{optional_label} must be string or null")

    workflow_sha = github.get("workflow_sha")
    if workflow_sha not in (None, "") and (
        not isinstance(workflow_sha, str) or OBJECT_ID_RE.fullmatch(workflow_sha.lower()) is None
    ):
        raise VerificationError("observation github.workflow_sha must be an object id or empty/null")
    os_release_observed = _require_string(
        "observation host.os_release_sha256", host.get("os_release_sha256")
    )
    if HEX64_RE.fullmatch(os_release_observed.lower()) is None:
        raise VerificationError("observation host.os_release_sha256 must be a SHA-256 digest")
    upload_commit = _require_string(
        "observation transport.upload_artifact_commit",
        transport.get("upload_artifact_commit"),
    )
    if OBJECT_ID_RE.fullmatch(upload_commit.lower()) is None:
        raise VerificationError(
            "observation transport.upload_artifact_commit must be an object id"
        )

    observation_repository = github.get("repository")
    if not isinstance(observation_repository, str):
        raise VerificationError("observation github.repository must be a string")
    _expect_equal(
        "identity/observation repository",
        repository,
        observation_repository.lower(),
    )

    cross_bindings = {
        "environment.arch": str(runner.get("arch", "")).lower(),
        "environment.kernel_release": host.get("kernel_release"),
        "source.os_release": host.get("os_release_sha256"),
        "tool.python": tools.get("python"),
    }
    git_observed = tools.get("git")
    if not isinstance(git_observed, str) or not git_observed.startswith("git version "):
        raise VerificationError("observation tools.git must use 'git version X' form")
    cross_bindings["tool.git"] = git_observed.removeprefix("git version ")

    for key, observed in cross_bindings.items():
        if key in bindings:
            _expect_equal(f"binding cross-check {key}", bindings[key], observed)

    _expect_equal("expected repository", repository, args.expected_repository)
    _expect_equal("expected commit", commit, args.expected_commit)
    _expect_equal("expected tree", tree, args.expected_tree)
    _expect_equal("expected workflow blob", workflow_blob, args.expected_workflow_blob)
    _expect_equal("expected workflow path", workflow_path, args.expected_workflow_path)
    _expect_equal("expected run ID", github.get("run_id"), args.expected_run_id)
    _expect_equal("expected run attempt", github.get("run_attempt"), args.expected_run_attempt)
    _expect_equal("expected event", github.get("event_name"), args.expected_event)
    _expect_equal(
        "expected upload-artifact commit",
        transport.get("upload_artifact_commit"),
        args.expected_upload_artifact_commit,
    )
    _expect_equal(
        "expected identity SHA-256", identity_digest, args.expected_identity_sha256
    )
    _expect_equal(
        "expected observation SHA-256",
        observation_digest,
        args.expected_observation_sha256,
    )

    return {
        "archive_sha256": archive_digest,
        "identity_sha256": identity_digest,
        "observation_sha256": observation_digest,
    }


def _hex64_arg(value: str) -> str:
    value = value.lower()
    if HEX64_RE.fullmatch(value) is None:
        raise argparse.ArgumentTypeError("must be exactly 64 lowercase/uppercase hex characters")
    return value


def _object_id_arg(value: str) -> str:
    value = value.lower()
    if OBJECT_ID_RE.fullmatch(value) is None:
        raise argparse.ArgumentTypeError("must be a 40- or 64-character object id")
    return value


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--expected-archive-sha256", type=_hex64_arg)
    parser.add_argument("--expected-identity-sha256", type=_hex64_arg)
    parser.add_argument("--expected-observation-sha256", type=_hex64_arg)
    parser.add_argument("--expected-repository", type=lambda value: value.lower())
    parser.add_argument("--expected-commit", type=_object_id_arg)
    parser.add_argument("--expected-tree", type=_object_id_arg)
    parser.add_argument("--expected-workflow-blob", type=_object_id_arg)
    parser.add_argument("--expected-workflow-path")
    parser.add_argument("--expected-run-id")
    parser.add_argument("--expected-run-attempt")
    parser.add_argument("--expected-event")
    parser.add_argument("--expected-upload-artifact-commit", type=_object_id_arg)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        result = verify_envelope(args)
    except VerificationError as error:
        raise SystemExit(f"ERROR: {error}") from error
    print(f"PASS identity_sha256={result['identity_sha256']}")
    print(f"PASS observation_sha256={result['observation_sha256']}")
    if result["archive_sha256"]:
        print(f"PASS archive_sha256={result['archive_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
