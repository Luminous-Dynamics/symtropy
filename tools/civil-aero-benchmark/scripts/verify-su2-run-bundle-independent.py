#!/usr/bin/env python3
"""Independent verifier for one composed SU2 dry-run/production evidence bundle."""
from __future__ import annotations

import csv
import hashlib
import math
import struct
import sys
from pathlib import Path


SCHEMA = "su2-verified-run-bundle/0.2.0"
EVIDENCE = (
    "replayed_dry_run_multi_field_protocol_and_native_solver_termination_classification_"
    "bound_to_production_history_screen_and_inputs_unsigned_wall_time_only_other_"
    "resources_unknown_not_grid_independent_or_experimentally_validated"
)
TERMINATION_SCHEMA = "su2-native-termination-evidence/0.1.0"
TERMINATION_EVIDENCE = (
    "receipt_bound_su2_solver_exit_reason_and_native_criterion_summary_from_exact_"
    "stdout_stderr_bytes_not_build_attested_not_independent_history_screen_or_physical_validation"
)
STEADY_SUCCESS = "All convergence criteria satisfied."
WINDOW_SUCCESS = "All windowed time-averaged convergence criteria are fullfilled."
INTERRUPTED = "Interrupt signal received, exiting before the convergence criteria were satisfied."
STEADY_ITERATION_LIMIT = "Maximum number of iterations reached"
TIME_LIMIT = "Maximum time reached"
TIME_ITERATION_LIMIT = "Maximum number of time iterations reached"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def add_string(buffer: bytearray, value: str) -> None:
    raw = value.encode("utf-8")
    buffer.extend(struct.pack("<Q", len(raw)))
    buffer.extend(raw)


def add_measure(buffer: bytearray, row: dict[str, str], prefix: str) -> None:
    status = row[f"{prefix}_status"]
    if status == "measured":
        value = float(row[f"{prefix}_value"])
        require(math.isfinite(value) and value >= 0.0, f"{prefix} value invalid")
        buffer.append(1)
        buffer.extend(struct.pack("<d", value))
        add_string(buffer, row[f"{prefix}_unit"])
        add_string(buffer, row[f"{prefix}_method"])
        source, uncertainty = row[f"{prefix}_source_uncertainty"].split(":", 1)
        require(len(source) == 64, f"{prefix} source digest malformed")
        add_string(buffer, source)
        if uncertainty:
            uncertainty_value = float(uncertainty)
            require(math.isfinite(uncertainty_value) and uncertainty_value >= 0.0,
                    f"{prefix} uncertainty invalid")
            buffer.append(1)
            buffer.extend(struct.pack("<d", uncertainty_value))
        else:
            buffer.append(0)
    elif status == "unknown":
        require(not row[f"{prefix}_value"] and not row[f"{prefix}_unit"]
                and not row[f"{prefix}_method"], f"{prefix} unknown measurement contains values")
        buffer.append(0)
        add_string(buffer, row[f"{prefix}_source_uncertainty"])
    else:
        raise ValueError(f"{prefix} has unsupported measurement status {status!r}")


def add_u64(buffer: bytearray, value: int) -> None:
    buffer.extend(struct.pack("<Q", value))


def termination_evidence(
    production_dir: Path,
    preflight: dict[str, str],
    receipt_row: dict[str, str],
) -> tuple[str, str]:
    stdout = (production_dir / "stdout.log").read_bytes()
    stderr = (production_dir / "stderr.log").read_bytes()
    stdout_sha = hashlib.sha256(stdout).hexdigest()
    stderr_sha = hashlib.sha256(stderr).hexdigest()
    require(stdout_sha == receipt_row["stdout_sha256"], "termination stdout receipt mismatch")
    require(stderr_sha == receipt_row["stderr_sha256"], "termination stderr receipt mismatch")
    combined = stdout.decode("utf-8") + "\n" + stderr.decode("utf-8")
    marker = "Solver Exit"
    positions = [index for index in range(len(combined)) if combined.startswith(marker, index)]
    require(len(positions) == 1, f"expected exactly one Solver Exit section, found {len(positions)}")
    section = combined[positions[0]:]
    section_sha = hashlib.sha256(section.encode("utf-8")).hexdigest()
    time_domain = preflight["time_domain"] == "YES"
    time_window = preflight["window_cauchy_enabled"] == "YES"
    require(time_window == time_domain,
            "verified run bundle only supports steady or explicit windowed time-domain protocols")
    native_rows: list[tuple[str, str, str, str]] = []
    if time_window:
        mode = "time_window"
        window_fields = preflight["window_convergence_fields"].split(";")
        if INTERRUPTED in section:
            classification = "interrupted"
            terminal_messages = [INTERRUPTED]
        elif WINDOW_SUCCESS in section:
            classification = "time_window_native_criteria_satisfied"
            terminal_messages = [WINDOW_SUCCESS]
            success_sha = hashlib.sha256(WINDOW_SUCCESS.encode("utf-8")).hexdigest()
            native_rows = [
                (field, field, "passed", success_sha) for field in window_fields
            ]
        elif TIME_LIMIT in section:
            classification = "maximum_time_reached"
            terminal_messages = [TIME_LIMIT]
        elif TIME_ITERATION_LIMIT in section:
            classification = "maximum_time_iterations_reached"
            terminal_messages = [TIME_ITERATION_LIMIT]
        else:
            classification = "unclassified_time_window_termination"
            terminal_messages = []
    else:
        mode = "steady"
        steady_fields = preflight["steady_convergence_fields"].split(";")
        if INTERRUPTED in section:
            classification = "interrupted"
            terminal_messages = [INTERRUPTED]
        elif STEADY_ITERATION_LIMIT in section:
            classification = "maximum_iterations_reached"
            terminal_messages = [STEADY_ITERATION_LIMIT]
        elif STEADY_SUCCESS in section:
            terminal_messages = [STEADY_SUCCESS]
            statuses = []
            for configured in steady_fields:
                cauchy_field = f"CAUCHY_{configured}"
                matches = []
                for line in section.splitlines():
                    cells = [cell.strip() for cell in line.split("|") if cell.strip()]
                    if not cells or cells[-1] not in {"Yes", "No"}:
                        continue
                    if configured in cells or cauchy_field in cells:
                        matches.append((line, cells[-1]))
                require(len(matches) == 1,
                        f"native SU2 convergence summary has {len(matches)} rows for {configured}")
                line, status = matches[0]
                reported_field = cauchy_field if cauchy_field in line else configured
                row_status = "passed" if status == "Yes" else "failed"
                row_sha = hashlib.sha256(line.encode("utf-8")).hexdigest()
                native_rows.append((configured, reported_field, row_status, row_sha))
                statuses.append(row_status)
            classification = (
                "steady_native_criteria_satisfied"
                if all(status == "passed" for status in statuses)
                else "steady_native_criteria_not_satisfied"
            )
        else:
            classification = "unclassified_steady_termination"
            terminal_messages = []

    canonical = bytearray()
    for value in (
        TERMINATION_SCHEMA,
        receipt_row["run_id"],
        receipt_row["solver_id"],
        receipt_row["solver_version_claim"],
        receipt_row["receipt_sha256"],
        receipt_row["executable_sha256"],
        stdout_sha,
        stderr_sha,
        section_sha,
        mode,
        classification,
        TERMINATION_EVIDENCE,
    ):
        add_string(canonical, value)
    add_u64(canonical, len(native_rows))
    for configured, reported, status, row_sha in native_rows:
        add_string(canonical, configured)
        add_string(canonical, reported)
        add_string(canonical, status)
        add_string(canonical, row_sha)
    add_u64(canonical, len(terminal_messages))
    for message in terminal_messages:
        add_string(canonical, message)
    digest = hashlib.sha256(canonical).hexdigest()
    require(classification in {
        "steady_native_criteria_satisfied",
        "time_window_native_criteria_satisfied",
    }, f"native SU2 termination is not converged: {classification}")
    require(all(row[2] == "passed" for row in native_rows),
            "native SU2 termination includes a failed criterion")
    return classification, digest


def receipt(run_directory: Path) -> dict[str, str]:
    rows = read_rows(run_directory / "runner_receipt.csv")
    require(len(rows) == 1, f"{run_directory}: expected one process receipt")
    row = rows[0]
    require(row["status"] in {
        "process_exited_zero_expected_outputs_present",
        "process_exited_zero_no_outputs_declared",
    }, f"{run_directory}: process receipt is not successful")
    invocation = run_directory / "invocation_manifest.txt"
    require(sha256(invocation) == row["invocation_manifest_sha256"],
            f"{run_directory}: invocation manifest digest mismatch")
    return row


def main() -> int:
    if len(sys.argv) != 9:
        raise ValueError(
            "usage: verify-su2-run-bundle-independent.py DRY_RUN_DIR PRODUCTION_DIR "
            "BUNDLE.csv PREFLIGHT.csv BOUNDARY.csv HISTORY.csv SCREEN.csv LINEAGE.csv"
        )
    dry_dir, production_dir = Path(sys.argv[1]), Path(sys.argv[2])
    bundle_rows = read_rows(Path(sys.argv[3]))
    preflight_rows = read_rows(Path(sys.argv[4]))
    boundary_rows = read_rows(Path(sys.argv[5]))
    history_rows = read_rows(Path(sys.argv[6]))
    screen_rows = read_rows(Path(sys.argv[7]))
    lineage_rows = read_rows(Path(sys.argv[8]))
    require(len(bundle_rows) == 1, "expected one run-bundle report row")
    require(len(preflight_rows) == 1, "expected one field-preflight report row")
    require(boundary_rows, "boundary audit report is empty")
    require(history_rows and screen_rows and lineage_rows, "component evidence report is empty")
    bundle, preflight = bundle_rows[0], preflight_rows[0]
    boundary = boundary_rows[0]
    history = history_rows[0]
    screen = screen_rows[0]
    lineage = lineage_rows[0]

    require(bundle["schema"] == SCHEMA, "run-bundle schema mismatch")
    require(bundle["evidence_level"] == EVIDENCE, "run-bundle evidence ceiling mismatch")
    dry_receipt = receipt(dry_dir)
    production_receipt = receipt(production_dir)
    require(dry_receipt["solver_id"] == "SU2_CFD_DRY_RUN", "wrong dry-run solver identity")
    require(production_receipt["solver_id"] == "SU2_CFD", "wrong production solver identity")
    require(bundle["dry_run_id"] == dry_receipt["run_id"], "dry-run ID mismatch")
    require(bundle["run_id"] == production_receipt["run_id"], "production run ID mismatch")
    require(bundle["dry_run_receipt_sha256"] == dry_receipt["receipt_sha256"],
            "dry-run receipt digest mismatch")
    require(bundle["production_receipt_sha256"] == production_receipt["receipt_sha256"],
            "production receipt digest mismatch")
    require(dry_receipt["executable_sha256"] == production_receipt["executable_sha256"]
            == bundle["executable_sha256"], "dry-run/production executable mismatch")

    for field in ("run_id", "solver_id", "runner_receipt_sha256", "executable_sha256"):
        expected = {
            "run_id": dry_receipt["run_id"],
            "solver_id": dry_receipt["solver_id"],
            "runner_receipt_sha256": dry_receipt["receipt_sha256"],
            "executable_sha256": dry_receipt["executable_sha256"],
        }[field]
        require(preflight[field] == expected, f"preflight {field} mismatch")
        require(boundary[field] == expected, f"boundary audit {field} mismatch")
    require(preflight["configuration_sha256"] == boundary["configuration_sha256"],
            "preflight/boundary configuration mismatch")
    require(preflight["mesh_sha256"] == boundary["mesh_sha256"],
            "preflight/boundary mesh mismatch")
    require(preflight["preflight_sha256"] == bundle["field_preflight_sha256"],
            "preflight digest differs from run bundle")
    require(boundary["audit_sha256"] == bundle["boundary_audit_sha256"],
            "boundary audit digest differs from run bundle")

    require(production_receipt["run_id"] == history["run_id"] == screen["run_id"]
            == lineage["run_id"], "production run IDs differ across components")
    require(production_receipt["receipt_sha256"] == history["runner_receipt_sha256"]
            == screen["runner_receipt_sha256"] == lineage["runner_receipt_sha256"],
            "production receipt digest differs across components")
    require(production_receipt["executable_sha256"] == history["executable_sha256"]
            == lineage["executable_sha256"], "production executable digest mismatch")
    require(bundle["history_sha256"] == history["history_sha256"] == screen["history_sha256"],
            "raw history digest mismatch")
    require(bundle["history_import_sha256"] == history["import_sha256"]
            == screen["history_import_sha256"] == lineage["history_import_sha256"],
            "history import digest mismatch")
    require(bundle["convergence_screen_sha256"] == screen["screen_sha256"]
            == lineage["convergence_screen_sha256"], "convergence screen digest mismatch")
    require(screen["overall_status"] == "declared_thresholds_met"
            and screen["criterion_status"] == "passed", "a declared screen criterion did not pass")

    termination_classification, termination_sha = termination_evidence(
        production_dir, preflight, production_receipt
    )
    require(bundle["termination_classification"] == termination_classification,
            "run-bundle native termination classification mismatch")
    require(bundle["termination_sha256"] == termination_sha,
            "run-bundle native termination digest mismatch")

    for row in boundary_rows:
        for key in ("run_id", "runner_receipt_sha256", "configuration_sha256", "mesh_sha256",
                    "audit_sha256", "status"):
            require(row[key] == boundary[key], f"boundary report rows disagree on {key}")
    for row in history_rows:
        require(row["run_id"] == production_receipt["run_id"], "history rows mix run IDs")
        require(row["history_sha256"] == bundle["history_sha256"], "history rows mix raw histories")
    for row in screen_rows:
        require(row["screen_sha256"] == bundle["convergence_screen_sha256"],
                "screen rows mix screen digests")
        require(row["overall_status"] == "declared_thresholds_met"
                and row["criterion_status"] == "passed", "screen contains a failed/incomplete criterion")
    for row in lineage_rows:
        require(row["run_id"] == production_receipt["run_id"], "lineage rows mix run IDs")
        require(row["binding_sha256"] == bundle["lineage_binding_sha256"],
                "lineage digest mismatch")
        require(row["executable_sha256"] == bundle["executable_sha256"],
                "lineage executable mismatch")

    # Re-hash the exact configuration, geometry, mesh and history bytes under production.
    roles = {row["artifact_role"]: row for row in lineage_rows}
    for role, report_field in (
        ("physics_configuration", "physics_configuration_sha256"),
        ("geometry", "geometry_sha256"),
        ("mesh", "mesh_sha256"),
        ("history", "history_sha256"),
    ):
        require(role in roles, f"lineage is missing {role}")
        artifact = roles[role]
        raw_path = production_dir / artifact["relative_path"]
        require(raw_path.is_file(), f"missing production artifact {artifact['relative_path']}")
        require(len(raw_path.read_bytes()) == int(artifact["byte_length"]),
                f"{role} byte length mismatch")
        require(sha256(raw_path) == artifact["artifact_sha256"] == bundle[report_field],
                f"{role} exact-byte digest mismatch")
    require(bundle["physics_configuration_sha256"] == preflight["configuration_sha256"],
            "preflight configuration differs from production bundle")
    require(bundle["mesh_sha256"] == preflight["mesh_sha256"],
            "preflight mesh differs from production bundle")

    # The only measured resource currently available is wall time in the process receipt.
    require(bundle["wall_time_status"] == "measured", "wall time must be receipt-derived")
    require(float(bundle["wall_time_value"]) == int(production_receipt["elapsed_milliseconds"]) / 1000.0,
            "wall time does not match process receipt")
    require(bundle["wall_time_unit"] == "s"
            and bundle["wall_time_method"] == "external_solver_receipt.elapsed_milliseconds",
            "wall time method/unit mismatch")
    wall_source, wall_uncertainty = bundle["wall_time_source_uncertainty"].split(":", 1)
    require(wall_source == production_receipt["receipt_sha256"] and not wall_uncertainty,
            "wall time source or uncertainty encoding mismatch")
    for prefix in ("cpu", "peak_memory", "gpu", "energy"):
        require(bundle[f"{prefix}_status"] == "unknown", f"{prefix} must remain unknown")
        require(bundle[f"{prefix}_source_uncertainty"], f"{prefix} unknown reason is missing")

    # Recompute resource evidence and complete bundle canonical digests independently.
    resource = bytearray()
    add_string(resource, "su2-resource-evidence/0.1.0")
    for prefix in ("wall_time", "cpu", "peak_memory", "gpu", "energy"):
        add_measure(resource, bundle, prefix)
    resource_digest = hashlib.sha256(resource).hexdigest()
    require(bundle["resource_evidence_sha256"] == resource_digest,
            "resource evidence canonical digest mismatch")

    canonical = bytearray()
    for field in (
        "schema", "run_id", "dry_run_id", "solver_id", "solver_version_claim",
        "executable_sha256", "dry_run_receipt_sha256", "production_receipt_sha256",
        "physics_configuration_sha256", "geometry_sha256", "mesh_sha256",
        "field_preflight_sha256", "boundary_audit_sha256", "history_sha256",
        "history_import_sha256", "convergence_screen_sha256", "lineage_binding_sha256",
        "termination_classification", "termination_sha256",
        "resource_evidence_sha256", "status", "evidence_level",
    ):
        add_string(canonical, bundle[field])
    for prefix in ("wall_time", "cpu", "peak_memory", "gpu", "energy"):
        add_measure(canonical, bundle, prefix)
    digest = hashlib.sha256(canonical).hexdigest()
    require(bundle["bundle_sha256"] == digest, "run-bundle canonical digest mismatch")

    print(
        "independent_su2_verified_run_bundle_oracle=pass "
        "dry_run=pass production=pass preflight=pass boundary=pass history=pass screen=pass "
        "lineage=pass exact_input_bytes=pass measured_resources=1 unknown_resources=4 "
        f"bundle_sha256={digest}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent SU2 verified run-bundle oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
