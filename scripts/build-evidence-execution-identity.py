#!/usr/bin/env python3
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Build a deterministic semantic identity for an evidence-bearing execution.

The identity intentionally excludes observation metadata such as GitHub run IDs,
run attempts, and individual runner instances. Those belong in an evidence
sidecar, not in the semantic identity of the experiment itself.

The declared execution environment is different: runner class, isolation model,
architecture, image family/version, and other environment properties can change
what code is capable of executing and therefore belong in `environment.*`
bindings when they are material to the experiment.

Callers may also declare a required binding contract. This makes omission of a
declared authority input fail closed, but it does not prove the declaration
itself is complete; reviewers must establish that closure against the workflow
and producer semantics.

For evidence workflows, `--assert-checkout` additionally proves that the supplied
commit, tree, and workflow blob match the live Git checkout and that no tracked,
untracked, or ignored workspace material is present. The CLI then requires its
output directory to be outside that checkout and rechecks coherence after writing
the identity files.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path, PurePosixPath

SCHEMA = "symtropy.evidence.execution-identity.v1"
OBJECT_ID_RE = re.compile(r"^[0-9a-fA-F]{40}(?:[0-9a-fA-F]{24})?$")
REPOSITORY_RE = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
BINDING_KEY_RE = re.compile(
    r"^(?:action|environment|input|policy|source|tool|producer|verifier)\.[a-z0-9][a-z0-9_.-]*$"
)
HASH_BINDING_NAMESPACES = {"action", "policy", "source", "producer", "verifier"}
FLOATING_GUARD_NAMESPACES = {"environment", "tool"}
FLOATING_ALIASES = {
    "current",
    "default",
    "head",
    "latest",
    "main",
    "master",
    "nightly",
    "stable",
    "tip",
    "trunk",
}
LATEST_SUFFIXES = ("-latest", "/latest", ":latest", "@latest")
FORBIDDEN_CONTROL = {"\x00", "\n", "\r"}
PRINTABLE_ASCII_RE = re.compile(r"^[\x20-\x7e]+$")


def _nonempty(name: str, value: str) -> str:
    if not value:
        raise ValueError(f"{name} must not be empty")
    if any(char in value for char in FORBIDDEN_CONTROL):
        raise ValueError(f"{name} contains a forbidden control character")
    return value


def _object_id(name: str, value: str) -> str:
    value = _nonempty(name, value)
    if not OBJECT_ID_RE.fullmatch(value):
        raise ValueError(f"{name} must be a 40- or 64-character hexadecimal object id")
    return value.lower()


def _semantic_value(name: str, namespace: str, value: str) -> str:
    value = _nonempty(name, value)
    if not PRINTABLE_ASCII_RE.fullmatch(value):
        raise ValueError(f"{name} must contain printable ASCII only")
    if value != value.strip():
        raise ValueError(f"{name} must not contain leading or trailing whitespace")

    normalized = value.lower()
    if namespace in FLOATING_GUARD_NAMESPACES and (
        normalized in FLOATING_ALIASES or normalized.endswith(LATEST_SUFFIXES)
    ):
        raise ValueError(
            f"{name} uses an obvious moving alias; bind a pinned semantic value "
            "and, where material, an immutable source/policy identity"
        )
    return value


def _repository(value: str) -> str:
    value = _nonempty("repository", value)
    if not REPOSITORY_RE.fullmatch(value):
        raise ValueError("repository must be in owner/name form")
    return value.lower()


def _workflow_path(value: str) -> str:
    value = _nonempty("workflow_path", value)
    if "\\" in value or value.startswith("/"):
        raise ValueError("workflow_path must be a repository-relative POSIX path")
    components = value.split("/")
    if any(part in {"", ".", ".."} for part in components):
        raise ValueError("workflow_path must not contain empty, '.' or '..' components")
    return PurePosixPath(*components).as_posix()


def _validate_binding_key(key: str, *, subject: str) -> str:
    if not BINDING_KEY_RE.fullmatch(key):
        raise ValueError(
            f"{subject} must use an authority-bearing namespace: "
            "action., environment., input., policy., source., tool., producer., or verifier."
        )
    return key


def _parse_bindings(raw_bindings: list[str]) -> dict[str, str]:
    bindings: dict[str, str] = {}
    for raw in raw_bindings:
        if "=" not in raw:
            raise ValueError(f"binding must be KEY=VALUE: {raw!r}")
        key, value = raw.split("=", 1)
        _validate_binding_key(key, subject="binding key")
        if key in bindings:
            raise ValueError(f"duplicate binding key: {key}")
        namespace = key.split(".", 1)[0]
        if namespace in HASH_BINDING_NAMESPACES:
            bindings[key] = _object_id(f"binding {key}", value)
        else:
            bindings[key] = _semantic_value(f"binding {key}", namespace, value)
    return bindings


def _parse_required_bindings(
    raw_required: list[str], bindings: dict[str, str]
) -> list[str]:
    required: set[str] = set()
    for key in raw_required:
        _validate_binding_key(key, subject="required binding key")
        if key in required:
            raise ValueError(f"duplicate required binding key: {key}")
        required.add(key)

    missing = sorted(required - bindings.keys())
    if missing:
        raise ValueError(
            "required binding is missing from supplied bindings: " + ", ".join(missing)
        )
    return sorted(required)


def _git(repository_root: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repository_root), *arguments],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "unknown git error"
        raise ValueError(f"git {' '.join(arguments)} failed: {detail}")
    return result.stdout.strip()


def _assert_checkout_identity(repository_root: Path, identity: dict[str, str]) -> None:
    repository_root = repository_root.resolve()
    actual_head = _object_id("live checkout HEAD", _git(repository_root, "rev-parse", "HEAD"))
    actual_tree = _object_id(
        "live checkout tree", _git(repository_root, "rev-parse", "HEAD^{tree}")
    )
    actual_workflow_blob = _object_id(
        "live workflow blob",
        _git(repository_root, "rev-parse", f"HEAD:{identity['workflow_path']}"),
    )

    if actual_head != identity["execution_commit"]:
        raise ValueError(
            "execution_commit does not match live checkout HEAD: "
            f"expected {identity['execution_commit']} got {actual_head}"
        )
    if actual_tree != identity["execution_tree"]:
        raise ValueError(
            "execution_tree does not match live checkout tree: "
            f"expected {identity['execution_tree']} got {actual_tree}"
        )
    if actual_workflow_blob != identity["workflow_blob"]:
        raise ValueError(
            "workflow_blob does not match workflow path at live checkout HEAD: "
            f"expected {identity['workflow_blob']} got {actual_workflow_blob}"
        )

    tracked_status = _git(repository_root, "status", "--porcelain=v1", "--untracked-files=all")
    extra_material = _git(repository_root, "clean", "-ndx")
    if tracked_status or extra_material:
        raise ValueError(
            "live checkout must contain only tracked HEAD material; "
            "tracked drift and ignored/untracked files are forbidden"
        )


def _assert_output_outside_checkout(repository_root: Path, output_dir: Path) -> None:
    root = repository_root.resolve()
    output = output_dir.resolve()
    try:
        output.relative_to(root)
    except ValueError:
        return
    raise ValueError("output_dir must be outside the asserted checkout")


def build_identity(args: argparse.Namespace) -> tuple[bytes, str]:
    bindings = _parse_bindings(args.binding)
    required = _parse_required_bindings(args.require_binding, bindings)
    identity = {
        "repository": _repository(args.repository),
        "execution_commit": _object_id("execution_commit", args.execution_commit),
        "execution_tree": _object_id("execution_tree", args.execution_tree),
        "workflow_path": _workflow_path(args.workflow_path),
        "workflow_blob": _object_id("workflow_blob", args.workflow_blob),
    }
    if getattr(args, "assert_checkout", False):
        repository_root = Path(getattr(args, "repository_root", Path(".")))
        _assert_checkout_identity(repository_root, identity)

    payload = {
        "schema": SCHEMA,
        "identity": identity,
        "binding_contract": {
            "required": required,
        },
        "bindings": bindings,
    }
    encoded = (
        json.dumps(
            payload,
            ensure_ascii=False,
            allow_nan=False,
            sort_keys=True,
            separators=(",", ":"),
        )
        + "\n"
    ).encode("utf-8")
    return encoded, hashlib.sha256(encoded).hexdigest()


def _atomic_write(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
        os.replace(temporary, path)
    except BaseException:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass
        raise


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--execution-commit", required=True)
    parser.add_argument("--execution-tree", required=True)
    parser.add_argument("--workflow-path", required=True)
    parser.add_argument("--workflow-blob", required=True)
    parser.add_argument(
        "--binding",
        action="append",
        default=[],
        metavar="KEY=VALUE",
        help=(
            "repeatable semantic binding using action.*, environment.*, input.*, policy.*, "
            "source.*, tool.*, producer.*, or verifier.*; "
            "action/policy/source/producer/verifier values must be immutable "
            "40- or 64-hex identities; free-form values must be review-safe printable ASCII"
        ),
    )
    parser.add_argument(
        "--require-binding",
        action="append",
        default=[],
        metavar="KEY",
        help=(
            "repeatable binding-closure declaration; every required key must also "
            "be supplied with --binding and the sorted required-key set is identity-bound"
        ),
    )
    parser.add_argument(
        "--assert-checkout",
        action="store_true",
        help=(
            "fail unless execution commit/tree/workflow blob match a checkout containing "
            "only tracked HEAD material; also requires output_dir outside that checkout"
        ),
    )
    parser.add_argument(
        "--repository-root",
        type=Path,
        default=Path("."),
        help="Git checkout root used by --assert-checkout (default: current directory)",
    )
    parser.add_argument("--output-dir", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.assert_checkout:
            _assert_output_outside_checkout(args.repository_root, args.output_dir)
        encoded, digest = build_identity(args)
    except ValueError as error:
        raise SystemExit(f"ERROR: {error}") from error

    identity_path = args.output_dir / "EXECUTION_IDENTITY.json"
    digest_path = args.output_dir / "EXECUTION_IDENTITY.sha256"
    _atomic_write(identity_path, encoded)
    _atomic_write(
        digest_path,
        f"{digest}  EXECUTION_IDENTITY.json\n".encode("ascii"),
    )

    if args.assert_checkout:
        payload = json.loads(encoded)
        try:
            _assert_checkout_identity(args.repository_root, payload["identity"])
        except ValueError as error:
            raise SystemExit(f"ERROR: post-write checkout assertion failed: {error}") from error

    print(digest)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
