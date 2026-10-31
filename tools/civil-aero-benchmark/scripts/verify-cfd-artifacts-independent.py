#!/usr/bin/env python3
"""Independent stdlib oracle for the on-disk CFD artifact manifest CLI."""
from __future__ import annotations

import csv
import hashlib
import subprocess
import sys
import tempfile
from pathlib import Path

SCHEMA = "civil-cfd-run-artifacts/0.1.0"
ROLES = [
    "solver_build",
    "physics_configuration",
    "geometry",
    "mesh",
    "run_log",
    "residual_history",
    "force_history",
    "field_output",
]
HEADER = "run_id,artifact_role,relative_path,byte_length,sha256"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def write_bundle(root: Path) -> tuple[Path, list[tuple[str, str, bytes]]]:
    records = []
    run_id = "synthetic-run-001"
    run_dir = root / "run-001"
    run_dir.mkdir(parents=True)
    for role in ROLES:
        payload = f"synthetic byte content for {role}\n".encode()
        relative_path = f"run-001/{role}.bin"
        (root / relative_path).write_bytes(payload)
        records.append((role, relative_path, payload))
    manifest = root / "manifest.csv"
    lines = [HEADER]
    for role, relative_path, payload in records:
        lines.append(f"{run_id},{role},{relative_path},{len(payload)},{sha256(payload)}")
    manifest.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return manifest, records


def execute(binary: Path, root: Path, manifest: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(binary), str(root), str(manifest)],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )


def verify_positive(output: str, records: list[tuple[str, str, bytes]]) -> None:
    rows = list(csv.DictReader(output.splitlines()))
    require(len(rows) == len(ROLES), "expected one output row per required artifact role")
    canonical = (
        f"schema={SCHEMA}\n"
        "run_id=synthetic-run-001\n"
        + "".join(
            f"{role}|{relative_path}|{len(payload)}|{sha256(payload)}\n"
            for role, relative_path, payload in records
        )
    ).encode()
    expected_manifest_sha = sha256(canonical)
    for row, (role, relative_path, payload) in zip(rows, records, strict=True):
        require(row["run_id"] == "synthetic-run-001", "run ID mismatch")
        require(row["canonical_manifest_sha256"] == expected_manifest_sha,
                "canonical manifest digest mismatch")
        require(int(row["artifact_count"]) == len(ROLES), "artifact count mismatch")
        require(row["artifact_role"] == role, "artifact role order/content mismatch")
        require(row["relative_path"] == relative_path, "artifact path mismatch")
        require(int(row["byte_length"]) == len(payload), "byte length mismatch")
        require(row["sha256"] == sha256(payload), "artifact digest mismatch")


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-cfd-artifacts-independent.py run_artifact_audit_binary",
              file=sys.stderr)
        return 2
    binary = Path(sys.argv[1]).resolve()
    require(binary.is_file(), "audit binary does not exist")
    with tempfile.TemporaryDirectory(prefix="civil-cfd-artifact-oracle-") as temp:
        root = Path(temp)
        manifest, records = write_bundle(root)

        positive = execute(binary, root, manifest)
        require(positive.returncode == 0,
                f"positive artifact fixture rejected: {positive.stderr}")
        require("byte_length_and_sha256_verified" in positive.stderr,
                "CLI did not emit its explicit evidence status")
        verify_positive(positive.stdout, records)

        # Same-length corruption must be rejected by a hash of actual file bytes.
        target = root / records[-1][1]
        mutated = bytearray(target.read_bytes())
        mutated[0] ^= 1
        target.write_bytes(mutated)
        tampered = execute(binary, root, manifest)
        require(tampered.returncode != 0, "same-length byte mutation unexpectedly passed")
        require("SHA-256 mismatch" in tampered.stderr,
                "same-length mutation failed for an unexpected reason")
        target.write_bytes(records[-1][2])

        # Path traversal is rejected before file access.
        original_manifest = manifest.read_text(encoding="utf-8")
        manifest.write_text(
            original_manifest.replace("run-001/solver_build.bin", "../solver_build.bin"),
            encoding="utf-8",
        )
        traversal = execute(binary, root, manifest)
        require(traversal.returncode != 0, "path traversal unexpectedly passed")
        require("safe root-relative path" in traversal.stderr,
                "path traversal rejected for an unexpected reason")
        manifest.write_text(original_manifest, encoding="utf-8")

        # Missing a required role is not treated as a partial success.
        manifest_lines = original_manifest.splitlines()
        manifest.write_text("\n".join(manifest_lines[:-1]) + "\n", encoding="utf-8")
        missing = execute(binary, root, manifest)
        require(missing.returncode != 0, "manifest missing a required role unexpectedly passed")
        require("exactly 8 required roles" in missing.stderr,
                "missing role rejected for an unexpected reason")

    print(
        "independent_cfd_artifact_oracle=pass roles=8 "
        "same_length_tamper=rejected path_traversal=rejected missing_role=rejected"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError, subprocess.SubprocessError) as error:
        print(f"independent_cfd_artifact_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
