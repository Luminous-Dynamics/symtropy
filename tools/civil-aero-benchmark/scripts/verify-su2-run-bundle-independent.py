#!/usr/bin/env python3
"""Independent verifier for one composed SU2 dry-run/production evidence bundle."""
from __future__ import annotations

import csv
import hashlib
import math
import struct
import sys
from tempfile import TemporaryDirectory
from pathlib import Path


SCHEMA = "su2-verified-run-bundle/0.3.0"
EVIDENCE = (
    "replayed_dry_run_multi_field_protocol_native_solver_termination_and_scheduler_signal_guard_"
    "and_every_saved_unsteady_time_step_inner_residual_screen_bound_to_production_history_"
    "and_inputs_unsigned_wall_time_only_other_resources_unknown_not_grid_independent_or_experimentally_validated"
)
TERMINATION_SCHEMA = "su2-native-termination-evidence/0.3.0"
TERMINATION_EVIDENCE = (
    "receipt_bound_su2_solver_exit_reason_per_field_steady_rows_or_aggregate_window_"
    "success_scoped_to_all_configured_fields_from_exact_full_stdout_stderr_bytes_"
    "not_build_attested_not_independent_history_screen_or_physical_validation"
)
STEADY_SUCCESS = "All convergence criteria satisfied."
WINDOW_SUCCESS = "All windowed time-averaged convergence criteria are fullfilled."
INTERRUPTED = "Interrupt signal received, exiting before the convergence criteria were satisfied."
SIGNAL_INTERRUPT_PREFIX = "Interrupt signal ("
SIGNAL_INTERRUPT_SUFFIX = ") received, saving files and exiting."
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


def scheduler_signal_message(full_log: str) -> str | None:
    for raw_line in full_log.splitlines():
        line = raw_line.strip()
        if line.startswith(SIGNAL_INTERRUPT_PREFIX):
            remainder = line[len(SIGNAL_INTERRUPT_PREFIX):]
            require(SIGNAL_INTERRUPT_SUFFIX in remainder,
                    "unrecognized SU2 signal-handler termination message")
            signal_number, trailing = remainder.split(SIGNAL_INTERRUPT_SUFFIX, 1)
            require(signal_number.isdigit() and not trailing.strip(),
                    "invalid SU2 signal number or trailing signal-handler text")
            return line
    return None


def self_test_scheduler_signal_classifier() -> None:
    successful_window_log = (
        "Interrupt signal (15) received, saving files and exiting.\n"
        "----------------------------- Solver Exit -------------------------------\n"
        "All windowed time-averaged convergence criteria are fullfilled.\n"
    )
    require(
        scheduler_signal_message(successful_window_log)
        == "Interrupt signal (15) received, saving files and exiting.",
        "scheduler signal before a successful window banner was not detected",
    )
    malformed = "Interrupt signal (fifteen) received, saving files and exiting."
    try:
        scheduler_signal_message(malformed)
    except ValueError:
        pass
    else:
        raise ValueError("malformed scheduler signal output was not rejected")

    # Exercise the full independent termination classifier: the signal precedes the
    # Solver Exit section, which itself contains a successful window-convergence banner.
    with TemporaryDirectory(prefix="su2-sigterm-self-test-") as temporary:
        run_directory = Path(temporary)
        stdout = (
            "Interrupt signal (15) received, saving files and exiting.\n"
            "----------------------------- Solver Exit -------------------------------\n"
            "All windowed time-averaged convergence criteria are fullfilled.\n"
        ).encode("utf-8")
        stderr = b""
        (run_directory / "stdout.log").write_bytes(stdout)
        (run_directory / "stderr.log").write_bytes(stderr)
        receipt = {
            "stdout_sha256": hashlib.sha256(stdout).hexdigest(),
            "stderr_sha256": hashlib.sha256(stderr).hexdigest(),
            "run_id": "sigterm-self-test",
            "solver_id": "SU2_CFD",
            "solver_version_claim": "synthetic-self-test",
            "receipt_sha256": "0" * 64,
            "executable_sha256": "1" * 64,
        }
        preflight = {
            "time_domain": "YES",
            "window_cauchy_enabled": "YES",
            "window_convergence_fields": "TAVG_DRAG",
        }
        try:
            termination_evidence(run_directory, preflight, receipt)
        except ValueError as error:
            require("interrupted" in str(error),
                    "signal plus successful window banner failed for an unexpected reason")
        else:
            raise ValueError("scheduler-interrupted window run was incorrectly admitted")

    # Exercise the full independent termination classifier: the signal precedes the
    # Solver Exit section, which itself contains a successful window-convergence banner.
    with TemporaryDirectory(prefix="su2-sigterm-self-test-") as temporary:
        run_directory = Path(temporary)
        stdout = (
            "Interrupt signal (15) received, saving files and exiting.\\n"
            "----------------------------- Solver Exit -------------------------------\\n"
            "All windowed time-averaged convergence criteria are fullfilled.\\n"
        ).encode("utf-8")
        stderr = b""
        (run_directory / "stdout.log").write_bytes(stdout)
        (run_directory / "stderr.log").write_bytes(stderr)
        receipt = {
            "stdout_sha256": hashlib.sha256(stdout).hexdigest(),
            "stderr_sha256": hashlib.sha256(stderr).hexdigest(),
            "run_id": "sigterm-self-test",
            "solver_id": "SU2_CFD",
            "solver_version_claim": "synthetic-self-test",
            "receipt_sha256": "0" * 64,
            "executable_sha256": "1" * 64,
        }
        preflight = {
            "time_domain": "YES",
            "window_cauchy_enabled": "YES",
            "window_convergence_fields": "TAVG_DRAG",
        }
        try:
            termination_evidence(run_directory, preflight, receipt)
        except ValueError as error:
            require("interrupted" in str(error),
                    "signal plus successful window banner failed for an unexpected reason")
        else:
            raise ValueError("scheduler-interrupted window run was incorrectly admitted")


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
    native_rows: list[tuple[str, str, str, str, str]] = []
    signal_message = scheduler_signal_message(combined)
    terminal_markers = (
        INTERRUPTED,
        STEADY_SUCCESS,
        WINDOW_SUCCESS,
        STEADY_ITERATION_LIMIT,
        TIME_LIMIT,
        TIME_ITERATION_LIMIT,
    )
    observed_terminal_markers = [
        marker for marker in terminal_markers if marker in section
    ]
    if signal_message is not None:
        conflicting_markers = [
            marker for marker in observed_terminal_markers if marker != INTERRUPTED
        ]
        mode = "steady" if not time_domain else ("time_window" if time_window else "unsteady_no_window")
        if conflicting_markers:
            classification = "contradictory_terminal_messages"
            terminal_messages = conflicting_markers + [signal_message]
        else:
            classification = "interrupted"
            terminal_messages = [signal_message]
    elif time_window:
        mode = "time_window"
        window_fields = preflight["window_convergence_fields"].split(";")
        if INTERRUPTED in section:
            other_markers = [
                marker for marker in observed_terminal_markers if marker != INTERRUPTED
            ]
            classification = (
                "contradictory_terminal_messages" if other_markers else "interrupted"
            )
            terminal_messages = other_markers + [INTERRUPTED]
        elif STEADY_SUCCESS in section or STEADY_ITERATION_LIMIT in section:
            classification = "contradictory_terminal_messages"
            terminal_messages = observed_terminal_markers
        elif WINDOW_SUCCESS in section:
            success_lines = [
                line for line in section.splitlines() if line.strip() == WINDOW_SUCCESS
            ]
            require(
                len(success_lines) == 1,
                f"expected exactly one native aggregate window-success line, found {len(success_lines)}",
            )
            success_line = success_lines[0]
            coincident_time_limit = TIME_LIMIT in section
            coincident_iteration_limit = TIME_ITERATION_LIMIT in section
            classification = (
                "time_window_native_criteria_satisfied_with_coincident_limit"
                if coincident_time_limit or coincident_iteration_limit
                else "time_window_native_criteria_satisfied"
            )
            terminal_messages = [success_line]
            if coincident_time_limit:
                terminal_messages.append(TIME_LIMIT)
            if coincident_iteration_limit:
                terminal_messages.append(TIME_ITERATION_LIMIT)
            success_sha = hashlib.sha256(success_line.encode("utf-8")).hexdigest()
            native_rows = [
                (
                    field,
                    "AGGREGATE_WINDOW_SUCCESS_BANNER",
                    "aggregate_window_success_banner",
                    "passed",
                    success_sha,
                )
                for field in window_fields
            ]
        elif TIME_LIMIT in section or TIME_ITERATION_LIMIT in section:
            coincident_time_limit = TIME_LIMIT in section
            coincident_iteration_limit = TIME_ITERATION_LIMIT in section
            classification = (
                "time_limits_reached_before_window_convergence"
                if coincident_time_limit and coincident_iteration_limit
                else "maximum_time_reached"
                if coincident_time_limit
                else "maximum_time_iterations_reached"
            )
            terminal_messages = []
            if coincident_time_limit:
                terminal_messages.append(TIME_LIMIT)
            if coincident_iteration_limit:
                terminal_messages.append(TIME_ITERATION_LIMIT)
        else:
            classification = "unclassified_time_window_termination"
            terminal_messages = observed_terminal_markers
    else:
        mode = "steady"
        steady_fields = preflight["steady_convergence_fields"].split(";")
        if len(observed_terminal_markers) > 1:
            classification = "contradictory_terminal_messages"
            terminal_messages = observed_terminal_markers
        elif INTERRUPTED in section:
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
                native_rows.append((
                    configured,
                    reported_field,
                    "per_field_native_summary_row",
                    row_status,
                    row_sha,
                ))
                statuses.append(row_status)
            classification = (
                "steady_native_criteria_satisfied"
                if all(status == "passed" for status in statuses)
                else "steady_native_criteria_not_satisfied"
            )
        else:
            classification = "unclassified_steady_termination"
            terminal_messages = observed_terminal_markers

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
    for configured, reported, evidence_scope, status, row_sha in native_rows:
        add_string(canonical, configured)
        add_string(canonical, reported)
        add_string(canonical, evidence_scope)
        add_string(canonical, status)
        add_string(canonical, row_sha)
    add_u64(canonical, len(terminal_messages))
    for message in terminal_messages:
        add_string(canonical, message)
    digest = hashlib.sha256(canonical).hexdigest()
    require(classification in {
        "steady_native_criteria_satisfied",
        "time_window_native_criteria_satisfied",
        "time_window_native_criteria_satisfied_with_coincident_limit",
    }, f"native SU2 termination is not converged: {classification}")
    require(all(row[3] == "passed" for row in native_rows),
            "native SU2 termination includes a failed criterion")
    if time_window:
        require(
            all(
                row[1] == "AGGREGATE_WINDOW_SUCCESS_BANNER"
                and row[2] == "aggregate_window_success_banner"
                for row in native_rows
            ),
            "window convergence must be represented by aggregate banner evidence, not fabricated per-field rows",
        )
    else:
        require(
            all(row[2] == "per_field_native_summary_row" for row in native_rows),
            "steady convergence requires per-field native table rows",
        )
    return classification, digest


def unsteady_inner_screen_evidence(
    raw_history_path: Path,
    preflight: dict[str, str],
    history_import_rows: list[dict[str, str]],
    screen_rows: list[dict[str, str]],
    receipt_row: dict[str, str],
    bundle: dict[str, str],
) -> str:
    if preflight["time_domain"] != "YES":
        require(not screen_rows, "steady run unexpectedly contains unsteady inner-screen rows")
        require(not bundle["unsteady_inner_screen_sha256"],
                "steady bundle unexpectedly contains an unsteady inner-screen digest")
        return ""

    schema = "su2-unsteady-inner-step-screen/0.1.0"
    evidence = (
        "receipt_and_history_bound_every_contiguous_saved_time_iter_checked_against_"
        "explicit_log10_residual_threshold_residual_fields_only_not_inner_solver_native_"
        "iteration_trace_or_temporal_grid_independent_or_experimentally_validated"
    )
    require(history_import_rows, "unsteady history import is empty")
    imported_first = history_import_rows[0]
    require(imported_first["iteration_column"] == "TIME_ITER",
            "unsteady imported history must use TIME_ITER")
    require(imported_first["runner_receipt_sha256"] == receipt_row["receipt_sha256"],
            "unsteady imported history receipt mismatch")
    require(imported_first["executable_sha256"] == receipt_row["executable_sha256"],
            "unsteady imported history executable mismatch")
    require(imported_first["history_sha256"] == bundle["history_sha256"],
            "unsteady imported history artifact mismatch")
    require(imported_first["import_sha256"] == bundle["history_import_sha256"],
            "unsteady imported history digest mismatch")

    unit_by_field: dict[str, str] = {}
    for key in imported_first:
        if key.endswith("]") and " [" in key:
            column, unit = key.rsplit(" [", 1)
            unit_by_field[column] = unit[:-1]
    require(unit_by_field.get("INNER_ITER") == "iterations",
            "unsteady import must include INNER_ITER [iterations]")
    raw_rows = read_rows(raw_history_path)
    require(len(raw_rows) >= 2, "unsteady raw history needs at least two saved time steps")
    raw_headers = set(raw_rows[0])
    require({"TIME_ITER", "INNER_ITER"} <= raw_headers,
            "unsteady raw history is missing TIME_ITER or INNER_ITER")

    fields = preflight["steady_convergence_fields"].split(";")
    require(fields and all(fields), "unsteady CONV_FIELD is empty")
    threshold_text = preflight["steady_residual_min_log10"]
    require(threshold_text, "unsteady inner screen requires explicit CONV_RESIDUAL_MINVAL")
    threshold = float(threshold_text)
    require(math.isfinite(threshold), "unsteady inner residual threshold is non-finite")
    require(preflight["window_cauchy_enabled"] == "YES",
            "unsteady inner screen requires enabled time-window convergence")
    window_start = int(preflight["window_start_iter"])
    convergence_start = int(preflight["window_convergence_start_iter"])
    window_elements = int(preflight["window_cauchy_elements"])
    require(window_elements > 0, "window Cauchy element count must be positive")
    required_terminal_iteration = window_start + max(convergence_start, window_elements)

    time_iterations: list[int] = []
    inner_iterations: list[int] = []
    for row in raw_rows:
        time_value = float(row["TIME_ITER"])
        inner_value = float(row["INNER_ITER"])
        require(time_value.is_integer() and time_value >= 0,
                "TIME_ITER must be a non-negative integer")
        require(inner_value.is_integer() and inner_value >= 1,
                "INNER_ITER must be a positive integer")
        time_iterations.append(int(time_value))
        inner_iterations.append(int(inner_value))
    require(all(
        right == left + 1 for left, right in zip(time_iterations, time_iterations[1:])
    ), "unsteady raw history has a TIME_ITER gap")
    require(time_iterations[0] <= window_start,
            "unsteady history begins after WINDOW_START_ITER")
    require(time_iterations[-1] >= required_terminal_iteration,
            "unsteady history ends before the configured window convergence period")
    require(len(history_import_rows) == len(raw_rows),
            "unsteady imported history sample count differs from raw history")
    inner_key = "INNER_ITER [iterations]"
    for index, (imported, raw) in enumerate(zip(history_import_rows, raw_rows)):
        require(int(imported["iteration"]) == time_iterations[index],
                f"unsteady imported TIME_ITER differs at row {index}")
        require(float(imported[inner_key]) == float(raw["INNER_ITER"]),
                f"unsteady imported INNER_ITER differs at row {index}")

    canonical = bytearray()
    screen_status = "all_contiguous_saved_time_steps_passed_inner_residual_thresholds"
    report_fields = []
    for field in fields:
        expected_unit = (
            "log10(relative residual)" if field.startswith("REL_RMS_")
            else "log10(residual)" if field.startswith("RMS_")
            else None
        )
        require(expected_unit is not None,
                f"unsteady inner screen supports residual fields only; unsupported {field}")
        require(unit_by_field.get(field) == expected_unit,
                f"unsteady import unit mismatch for {field}")
        require(field in raw_headers, f"unsteady raw history is missing {field}")
        import_key = f"{field} [{expected_unit}]"
        for index, (imported, raw) in enumerate(zip(history_import_rows, raw_rows)):
            require(float(imported[import_key]) == float(raw[field]),
                    f"unsteady imported {field} differs from raw history at row {index}")
        values = [float(row[field]) for row in raw_rows]
        require(all(math.isfinite(value) for value in values),
                f"unsteady residual {field} contains a non-finite value")
        maximum = max(values)
        worst_index = values.index(maximum)
        failed = sum(value > threshold for value in values)
        require(failed == 0,
                f"unsteady residual {field} exceeded threshold in {failed} saved time steps")
        report_fields.append({
            "field": field,
            "unit": expected_unit,
            "threshold_log10": threshold,
            "maximum_observed_log10": maximum,
            "worst_time_iteration": time_iterations[worst_index],
            "failed_time_step_count": failed,
        })

    # Verify the independently exported per-field screen rows before reconstructing its digest.
    require(len(screen_rows) == len(report_fields),
            "unsteady inner-screen field row count mismatch")
    common_fields = (
        "schema", "run_id", "solver_id", "runner_receipt_sha256", "executable_sha256",
        "history_sha256", "history_import_sha256", "preflight_sha256",
        "time_iteration_column", "inner_iteration_column", "first_time_iteration",
        "last_time_iteration", "contiguous_time_step_count", "status", "screen_sha256",
        "evidence_level",
    )
    expected_common = {
        "schema": schema,
        "run_id": receipt_row["run_id"],
        "solver_id": receipt_row["solver_id"],
        "runner_receipt_sha256": receipt_row["receipt_sha256"],
        "executable_sha256": receipt_row["executable_sha256"],
        "history_sha256": bundle["history_sha256"],
        "history_import_sha256": bundle["history_import_sha256"],
        "preflight_sha256": bundle["field_preflight_sha256"],
        "time_iteration_column": "TIME_ITER",
        "inner_iteration_column": "INNER_ITER",
        "first_time_iteration": str(time_iterations[0]),
        "last_time_iteration": str(time_iterations[-1]),
        "contiguous_time_step_count": str(len(raw_rows)),
        "status": screen_status,
        "evidence_level": evidence,
    }
    for index, (row, expected) in enumerate(zip(screen_rows, report_fields)):
        for key in common_fields:
            require(row[key] == expected_common[key], f"unsteady screen row {index} has wrong {key}")
        require(row["field"] == expected["field"], "unsteady screen field ordering mismatch")
        require(row["unit"] == expected["unit"], "unsteady screen unit mismatch")
        require(float(row["threshold_log10"]) == expected["threshold_log10"],
                "unsteady screen threshold mismatch")
        require(float(row["maximum_observed_log10"]) == expected["maximum_observed_log10"],
                "unsteady screen maximum residual mismatch")
        require(int(row["worst_time_iteration"]) == expected["worst_time_iteration"],
                "unsteady screen worst time iteration mismatch")
        require(int(row["failed_time_step_count"]) == 0,
                "unsteady screen reports a failed time step")

    for value in (
        schema,
        receipt_row["run_id"],
        receipt_row["solver_id"],
        receipt_row["receipt_sha256"],
        receipt_row["executable_sha256"],
        bundle["history_sha256"],
        bundle["history_import_sha256"],
        bundle["field_preflight_sha256"],
        "TIME_ITER",
        "INNER_ITER",
        screen_status,
        evidence,
    ):
        add_string(canonical, value)
    add_u64(canonical, time_iterations[0])
    add_u64(canonical, time_iterations[-1])
    add_u64(canonical, len(raw_rows))
    add_u64(canonical, len(report_fields))
    for row in report_fields:
        add_string(canonical, row["field"])
        add_string(canonical, row["unit"])
        canonical.extend(struct.pack("<d", row["threshold_log10"]))
        canonical.extend(struct.pack("<d", row["maximum_observed_log10"]))
        add_u64(canonical, row["worst_time_iteration"])
        add_u64(canonical, row["failed_time_step_count"])
    digest = hashlib.sha256(canonical).hexdigest()
    require(all(row["screen_sha256"] == digest for row in screen_rows),
            "unsteady inner-screen canonical digest mismatch")
    require(bundle["unsteady_inner_screen_sha256"] == digest,
            "run-bundle unsteady inner-screen digest mismatch")
    return digest


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
    self_test_scheduler_signal_classifier()
    if len(sys.argv) != 10:
        raise ValueError(
            "usage: verify-su2-run-bundle-independent.py DRY_RUN_DIR PRODUCTION_DIR "
            "BUNDLE.csv PREFLIGHT.csv BOUNDARY.csv HISTORY.csv SCREEN.csv LINEAGE.csv "
            "UNSTEADY_INNER_SCREEN.csv"
        )
    dry_dir, production_dir = Path(sys.argv[1]), Path(sys.argv[2])
    bundle_rows = read_rows(Path(sys.argv[3]))
    preflight_rows = read_rows(Path(sys.argv[4]))
    boundary_rows = read_rows(Path(sys.argv[5]))
    history_rows = read_rows(Path(sys.argv[6]))
    screen_rows = read_rows(Path(sys.argv[7]))
    lineage_rows = read_rows(Path(sys.argv[8]))
    unsteady_inner_rows = read_rows(Path(sys.argv[9]))
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
    require(
        bundle["status"]
        == "run_identity_preflight_boundary_history_native_termination_and_required_inner_residual_screen_replayed",
        "run-bundle status does not represent all required protocol evidence",
    )
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
    raw_history_path = production_dir / roles["history"]["relative_path"]
    inner_digest = unsteady_inner_screen_evidence(
        raw_history_path,
        preflight,
        history_rows,
        unsteady_inner_rows,
        production_receipt,
        bundle,
    )

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
        "termination_classification", "termination_sha256", "unsteady_inner_screen_sha256",
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
        f"lineage=pass exact_input_bytes=pass unsteady_inner={'pass' if inner_digest else 'not_applicable'} "
        f"measured_resources=1 unknown_resources=4 bundle_sha256={digest}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent SU2 verified run-bundle oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
