#!/usr/bin/env python3
"""Independent oracle for the SU2 process-to-history-to-screen lineage binding."""
from __future__ import annotations

import csv
import hashlib
import re
import struct
import sys
from pathlib import Path


SCHEMA = "su2-run-lineage-binding/0.1.0"
EVIDENCE = (
    "replayed_process_receipt_bound_to_history_screen_and_staged_inputs_unsigned_"
    "not_build_source_attested_not_grid_admitted_not_experimentally_validated"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream))


def add_string(buffer: bytearray, value: str) -> None:
    raw = value.encode("utf-8")
    buffer.extend(struct.pack("<Q", len(raw)))
    buffer.extend(raw)


def add_u64(buffer: bytearray, value: int) -> None:
    buffer.extend(struct.pack("<Q", value))


def add_artifact(buffer: bytearray, artifact: dict[str, str]) -> None:
    add_string(buffer, artifact["relative_path"])
    add_u64(buffer, int(artifact["byte_length"]))
    add_string(buffer, artifact["artifact_sha256"])


def parse_manifest(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" not in line:
            raise ValueError("invocation manifest contains a malformed line")
        key, value = line.split("=", 1)
        require(key not in result, f"duplicate invocation manifest key {key}")
        result[key] = value
    return result


def main() -> int:
    if len(sys.argv) != 5:
        raise ValueError(
            "usage: verify-su2-run-lineage-independent.py RUN_DIR HISTORY_IMPORT_CSV SCREEN_CSV LINEAGE_CSV MESH_SUMMARY_CSV"
        )
    run = Path(sys.argv[1])
    history_import = rows(Path(sys.argv[2]))
    screen = rows(Path(sys.argv[3]))
    binding_rows = rows(Path(sys.argv[4]))
    mesh_summary_rows = rows(Path(sys.argv[5]))
    receipt_rows = rows(run / "runner_receipt.csv")
    output_rows = rows(run / "expected_outputs.csv")
    require(len(receipt_rows) == 1, "expected exactly one runner receipt")
    require(len(binding_rows) == 6, f"expected six lineage artifact rows, got {len(binding_rows)}")
    require(len(history_import) == 8, "expected eight imported history rows")
    require(len(screen) == 4, "expected four convergence criteria")
    require(len(mesh_summary_rows) == 1, "expected one independently checked mesh summary")
    mesh_summary = mesh_summary_rows[0]
    receipt = receipt_rows[0]
    common = binding_rows[0]

    require(common["schema"] == SCHEMA, "lineage schema mismatch")
    require(common["evidence_level"] == EVIDENCE, "lineage evidence ceiling mismatch")
    require(receipt["status"] == "process_exited_zero_expected_outputs_present", "process did not exit successfully")
    for field in ("run_id", "solver_id", "solver_version_claim", "executable_sha256", "receipt_sha256"):
        require(common[field] == receipt[field], f"lineage {field} differs from runner receipt")
    require(receipt["solver_id"] == "SU2_CFD", "wrong solver ID")
    require(re.fullmatch(r"[0-9a-f]{64}", receipt["executable_sha256"]) is not None, "executable digest malformed")
    require(re.fullmatch(r"[0-9a-f]{64}", receipt["receipt_sha256"]) is not None, "receipt digest malformed")

    invocation_bytes = (run / "invocation_manifest.txt").read_bytes()
    require(
        hashlib.sha256(invocation_bytes).hexdigest() == receipt["invocation_manifest_sha256"],
        "invocation manifest digest mismatch",
    )
    invocation = parse_manifest(run / "invocation_manifest.txt")
    require(invocation["run_id_utf8_hex"] == receipt["run_id"].encode().hex(), "invocation run ID mismatch")
    require(invocation["solver_id_utf8_hex"] == receipt["solver_id"].encode().hex(), "invocation solver ID mismatch")
    require(invocation["executable_sha256"] == receipt["executable_sha256"], "invocation executable digest mismatch")
    staged = {}
    for index in range(int(invocation["staged_input_count"])):
        path = bytes.fromhex(invocation[f"input_{index}_path_utf8_hex"]).decode()
        length = int(invocation[f"input_{index}_byte_length"])
        digest = invocation[f"input_{index}_sha256"]
        raw = (run / path).read_bytes()
        require(len(raw) == length, f"staged input length mismatch: {path}")
        require(hashlib.sha256(raw).hexdigest() == digest, f"staged input digest mismatch: {path}")
        staged[path] = {"relative_path": path, "byte_length": str(length), "artifact_sha256": digest}
    require(set(staged) == {"config.cfg", "geometry.dat", "mesh.su2"}, "staged input path set mismatch")

    outputs = {}
    for record in output_rows:
        require(record["relative_path"] not in outputs, "duplicate expected output path")
        outputs[record["relative_path"]] = record
        if record["status"] == "present":
            raw = (run / record["relative_path"]).read_bytes()
            require(len(raw) == int(record["byte_length"]), "expected output byte length mismatch")
            require(hashlib.sha256(raw).hexdigest() == record["sha256"], "expected output digest mismatch")
    require(set(outputs) == {"history.csv", "forces_breakdown.dat", "surface_flow.csv"}, "expected output set mismatch")
    require(all(outputs[path]["status"] == "present" for path in outputs), "a required output is missing")

    by_role: dict[str, dict[str, str]] = {}
    for row in binding_rows:
        require(row["run_id"] == receipt["run_id"], "lineage rows mix run IDs")
        for field in (
            "solver_id", "solver_version_claim", "executable_sha256", "runner_receipt_sha256",
            "history_import_sha256", "convergence_screen_sha256", "convergence_criterion_count",
            "convergence_passed_count", "binding_sha256", "evidence_level",
        ):
            require(row[field] == common[field], f"lineage rows disagree on {field}")
        role = row["artifact_role"]
        key = f"{role}:{row['relative_path']}"
        require(key not in by_role, "duplicate role/path row")
        by_role[key] = row
        require(re.fullmatch(r"[0-9a-f]{64}", row["artifact_sha256"]) is not None, "artifact digest malformed")

    roles = {row["artifact_role"] for row in binding_rows}
    require(roles == {"physics_configuration", "geometry", "mesh", "history", "additional_output"},
            f"lineage role set mismatch: {roles}")
    fixed = {}
    for role in ("physics_configuration", "geometry", "mesh", "history"):
        role_rows = [row for row in binding_rows if row["artifact_role"] == role]
        require(len(role_rows) == 1, f"expected one {role} row")
        fixed[role] = role_rows[0]
    expected_paths = {
        "physics_configuration": "config.cfg",
        "geometry": "geometry.dat",
        "mesh": "mesh.su2",
        "history": "history.csv",
    }
    for role, path in expected_paths.items():
        record = fixed[role]
        require(record["relative_path"] == path, f"{role} path mismatch")
        expected = staged[path] if role != "history" else outputs[path]
        expected_digest = expected["artifact_sha256"] if role != "history" else expected["sha256"]
        require(record["artifact_sha256"] == expected_digest, f"{role} digest mismatch")
        expected_length = expected["byte_length"]
        require(record["byte_length"] == expected_length, f"{role} byte length mismatch")
    for name in (
        "mesh_dimension",
        "mesh_point_count",
        "mesh_volume_element_count",
        "mesh_marker_count",
        "mesh_boundary_element_count",
        "mesh_summary_sha256",
    ):
        require(all(row[name] == common[name] for row in binding_rows), f"lineage rows disagree on {name}")
    for binding_name, summary_name in (
        ("mesh_dimension", "dimension"),
        ("mesh_point_count", "point_count"),
        ("mesh_volume_element_count", "volume_element_count"),
        ("mesh_marker_count", "marker_count"),
        ("mesh_boundary_element_count", "boundary_element_count"),
    ):
        require(common[binding_name] == mesh_summary[summary_name], f"mesh summary {binding_name} mismatch")
    mesh_summary_digest = hashlib.sha256(
        Path(sys.argv[5]).read_bytes()
    ).hexdigest()
    # The summary CSV is a single deterministic row and the source mesh digest is included within it.
    require(mesh_summary["mesh_sha256"] == fixed["mesh"]["artifact_sha256"], "parsed mesh summary source digest mismatch")
    require(
        common["mesh_summary_sha256"]
        == hashlib.sha256(
            (
                # Canonical summary CSV equals the complete one-row report plus a trailing newline.
                Path(sys.argv[5]).read_text(encoding="utf-8")
            ).encode("utf-8")
        ).hexdigest(),
        "mesh summary digest mismatch",
    )
    additional = sorted(
        (row for row in binding_rows if row["artifact_role"] == "additional_output"),
        key=lambda row: row["relative_path"],
    )
    require({row["relative_path"] for row in additional} == {"forces_breakdown.dat", "surface_flow.csv"},
            "additional output path set mismatch")
    for row in additional:
        output = outputs[row["relative_path"]]
        require(row["artifact_sha256"] == output["sha256"], "additional output digest mismatch")
        require(row["byte_length"] == output["byte_length"], "additional output length mismatch")

    for row in history_import:
        require(row["run_id"] == receipt["run_id"], "history import run ID mismatch")
        require(row["runner_receipt_sha256"] == receipt["receipt_sha256"], "history import receipt mismatch")
        require(row["executable_sha256"] == receipt["executable_sha256"], "history import executable mismatch")
        require(row["history_sha256"] == fixed["history"]["artifact_sha256"], "history import output mismatch")
        require(row["import_sha256"] == common["history_import_sha256"], "history import digest mismatch")
    for row in screen:
        require(row["run_id"] == receipt["run_id"], "screen run ID mismatch")
        require(row["runner_receipt_sha256"] == receipt["receipt_sha256"], "screen receipt mismatch")
        require(row["history_sha256"] == fixed["history"]["artifact_sha256"], "screen history mismatch")
        require(row["history_import_sha256"] == common["history_import_sha256"], "screen import mismatch")
        require(row["screen_sha256"] == common["convergence_screen_sha256"], "screen digest mismatch")
        require(row["overall_status"] == "declared_thresholds_met", "screen did not pass")
        require(row["criterion_status"] == "passed", "a declared criterion failed")

    # The Rust binding hash is a canonical length-prefixed binary record.
    buffer = bytearray()
    for value in [
        SCHEMA,
        receipt["run_id"],
        receipt["solver_id"],
        receipt["solver_version_claim"],
        receipt["executable_sha256"],
        receipt["receipt_sha256"],
        common["history_import_sha256"],
        common["convergence_screen_sha256"],
        EVIDENCE,
    ]:
        add_string(buffer, value)
    for role in ("physics_configuration", "geometry", "mesh", "history"):
        add_artifact(buffer, fixed[role])
    add_u64(buffer, int(common["mesh_dimension"]))
    add_u64(buffer, int(common["mesh_point_count"]))
    add_u64(buffer, int(common["mesh_volume_element_count"]))
    add_u64(buffer, int(common["mesh_marker_count"]))
    add_u64(buffer, int(common["mesh_boundary_element_count"]))
    add_string(buffer, common["mesh_summary_sha256"])
    add_u64(buffer, int(common["convergence_criterion_count"]))
    add_u64(buffer, int(common["convergence_passed_count"]))
    add_u64(buffer, len(additional))
    for row in additional:
        add_artifact(buffer, row)
    digest = hashlib.sha256(buffer).hexdigest()
    require(all(row["binding_sha256"] == digest for row in binding_rows), "canonical run-lineage digest mismatch")
    require(common["convergence_criterion_count"] == "4" and common["convergence_passed_count"] == "4",
            "convergence criteria counts mismatch")
    print(
        "independent_su2_run_lineage_oracle=pass artifacts=6 staged_inputs=3 "
        "history_rows=8 convergence_criteria=4 mesh_summary=pass receipt_replay=pass binding_sha256=" + digest
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent SU2 run-lineage oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
