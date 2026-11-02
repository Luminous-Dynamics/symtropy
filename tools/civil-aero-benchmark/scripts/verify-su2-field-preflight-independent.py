#!/usr/bin/env python3
"""Independent receipt/config/mesh and canonical oracle for SU2 convergence preflight v0.2."""
from __future__ import annotations

import csv
import hashlib
import math
import struct
import sys
from pathlib import Path

EVIDENCE = (
    "receipt_bound_su2_dry_run_multi_field_steady_and_explicit_time_window_"
    "configuration_discovery_only_not_history_convergence_or_validation"
)
SCHEMA = "su2-field-preflight/0.2.0"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream))


def add_string(buffer: bytearray, value: str) -> None:
    raw = value.encode("utf-8")
    buffer.extend(struct.pack("<Q", len(raw)))
    buffer.extend(raw)


def add_u64(buffer: bytearray, value: int) -> None:
    buffer.extend(struct.pack("<Q", value))


def add_optional_u64(buffer: bytearray, value: int | None) -> None:
    if value is None:
        buffer.append(0)
    else:
        buffer.append(1)
        add_u64(buffer, value)


def add_optional_string(buffer: bytearray, value: str | None) -> None:
    if value is None:
        buffer.append(0)
    else:
        buffer.append(1)
        add_string(buffer, value)


def independent_inventory(stdout: str, stderr: str) -> list[str]:
    combined = stdout + "\n" + stderr
    lines = combined.splitlines()
    marker = "available screen/history output fields for the current configuration"
    starts = [i for i, line in enumerate(lines) if marker in line.lower()]
    require(len(starts) == 1, "expected exactly one available history-field inventory")
    fields: set[str] = set()
    for line in lines[starts[0] + 1:]:
        stripped = line.strip()
        lower = stripped.lower()
        if lower.startswith("available ") and "output fields" in lower:
            break
        if "|" in stripped:
            stripped = stripped.strip("|").split("|", 1)[0].strip()
        for token in stripped.replace(",", " ").split():
            token = token.strip("'\"(),:;")
            if token and token[0].isupper() and all(
                character.isalnum() or character in "_-" for character in token
            ):
                if token not in {"Field", "Name", "FIELD", "NAME"}:
                    fields.add(token)
    return sorted(fields)


def parse_config(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    logical = ""
    for line_number, raw in enumerate(path.read_text(encoding="ascii").splitlines(), 1):
        line = raw.split("%", 1)[0].strip()
        if not line:
            continue
        if logical:
            continued = line.endswith("\\")
            logical += " " + (line[:-1].strip() if continued else line)
            if continued:
                continue
            line, logical = logical, ""
        elif line.endswith("\\"):
            logical = line[:-1].strip()
            continue
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        key, value = key.strip(), value.strip()
        if key in {
            "SOLVER", "CONV_FIELD", "CONV_RESIDUAL_MINVAL", "CONV_STARTITER",
            "CONV_CAUCHY_EPS", "CONV_CAUCHY_ELEMS", "HISTORY_OUTPUT",
            "WINDOW_CAUCHY_CRIT", "CONV_WINDOW_FIELD", "WINDOW_START_ITER",
            "CONV_WINDOW_STARTITER", "CONV_WINDOW_CAUCHY_EPS",
            "CONV_WINDOW_CAUCHY_ELEMS", "WINDOW_FUNCTION", "RESTART_ITER",
        }:
            require(key not in values, f"duplicate config key {key} at line {line_number}")
            values[key] = value
    require(not logical, "unterminated SU2 config continuation")
    return values


def field_list(raw: str, key: str) -> list[str]:
    normalized = raw.strip()
    if normalized.startswith("(") and normalized.endswith(")"):
        normalized = normalized[1:-1]
    fields = [item.strip().strip('"').strip() for item in normalized.split(",")]
    require(fields and all(fields), f"{key} has an empty field")
    require(len(fields) == len(set(fields)), f"{key} has duplicate fields")
    return fields


def optional_int(values: dict[str, str], key: str) -> int | None:
    raw = values.get(key)
    if raw is None:
        return None
    value = int(raw)
    require(value >= 0, f"{key} must be non-negative")
    return value


def optional_positive_decimal(values: dict[str, str], key: str) -> str | None:
    raw = values.get(key)
    if raw is None:
        return None
    value = float(raw)
    require(math.isfinite(value) and value > 0.0, f"{key} must be finite and positive")
    return raw


def main() -> int:
    if len(sys.argv) != 3:
        raise ValueError("usage: verify-su2-field-preflight-independent.py RUN_DIR REPORT_CSV")
    run = Path(sys.argv[1])
    report_rows = read_csv(Path(sys.argv[2]))
    receipt_rows = read_csv(run / "runner_receipt.csv")
    require(len(report_rows) == 1, "expected exactly one preflight report row")
    require(len(receipt_rows) == 1, "expected exactly one process receipt row")
    report, receipt = report_rows[0], receipt_rows[0]

    require(report["schema"] == SCHEMA, "preflight schema mismatch")
    require(report["evidence_level"] == EVIDENCE, "preflight evidence ceiling mismatch")
    require(
        report["status"] == "all_declared_steady_and_window_convergence_fields_recognized",
        "preflight did not pass",
    )
    require(receipt["solver_id"] == "SU2_CFD_DRY_RUN", "wrong solver ID")
    require(receipt["status"] == "process_exited_zero_no_outputs_declared", "dry-run process-only status mismatch")
    require(report["run_id"] == receipt["run_id"], "run ID mismatch")
    require(report["solver_id"] == receipt["solver_id"], "solver ID mismatch")
    require(report["runner_receipt_sha256"] == receipt["receipt_sha256"], "receipt digest mismatch")
    require(report["executable_sha256"] == receipt["executable_sha256"], "executable digest mismatch")

    config_path = run / report["configuration_path"]
    mesh_path = run / report["mesh_path"]
    config = config_path.read_bytes()
    mesh = mesh_path.read_bytes()
    config_sha = hashlib.sha256(config).hexdigest()
    mesh_sha = hashlib.sha256(mesh).hexdigest()
    require(len(config) == int(report["configuration_byte_length"]), "config length mismatch")
    require(config_sha == report["configuration_sha256"], "config digest mismatch")
    require(len(mesh) == int(report["mesh_byte_length"]), "mesh length mismatch")
    require(mesh_sha == report["mesh_sha256"], "mesh digest mismatch")

    stdout = (run / "stdout.log").read_bytes()
    stderr = (run / "stderr.log").read_bytes()
    stdout_sha = hashlib.sha256(stdout).hexdigest()
    stderr_sha = hashlib.sha256(stderr).hexdigest()
    require(stdout_sha == report["stdout_sha256"] == receipt["stdout_sha256"], "stdout digest mismatch")
    require(stderr_sha == report["stderr_sha256"] == receipt["stderr_sha256"], "stderr digest mismatch")

    invocation = {}
    for line in (run / "invocation_manifest.txt").read_text(encoding="utf-8").splitlines():
        key, value = line.split("=", 1)
        require(key not in invocation, f"duplicate invocation key {key}")
        invocation[key] = value
    require(invocation.get("argument_count") == "2", "expected two dry-run argv elements")
    args = [
        bytes.fromhex(invocation[f"argument_{index}_utf8_hex"]).decode("utf-8")
        for index in range(2)
    ]
    require(args == ["-d", report["configuration_path"]], f"wrong dry-run argv: {args}")

    config_values = parse_config(config_path)
    require(report["solver_family"] == config_values["SOLVER"].strip(), "solver family mismatch")
    steady_fields = field_list(config_values["CONV_FIELD"], "CONV_FIELD")
    require(report["configured_convergence_field"] == steady_fields[0], "legacy CONV_FIELD value mismatch")
    require(report["steady_convergence_fields"].split(";") == steady_fields, "steady field-list mismatch")

    steady_start = optional_int(config_values, "CONV_STARTITER")
    residual_min = config_values.get("CONV_RESIDUAL_MINVAL")
    if residual_min is not None:
        require(math.isfinite(float(residual_min)), "CONV_RESIDUAL_MINVAL is not finite")
    steady_eps = optional_positive_decimal(config_values, "CONV_CAUCHY_EPS")
    steady_elems = optional_int(config_values, "CONV_CAUCHY_ELEMS")
    if steady_elems is not None:
        require(steady_elems > 0, "CONV_CAUCHY_ELEMS must be positive")
    require(report["steady_start_iteration"] == ("" if steady_start is None else str(steady_start)),
            "steady start iteration mismatch")
    require(report["steady_residual_min_log10"] == ("" if residual_min is None else residual_min),
            "steady residual threshold mismatch")
    require(report["steady_cauchy_epsilon"] == ("" if steady_eps is None else steady_eps),
            "steady Cauchy epsilon mismatch")
    require(report["steady_cauchy_elements"] == ("" if steady_elems is None else str(steady_elems)),
            "steady Cauchy elements mismatch")

    window_mode = config_values["WINDOW_CAUCHY_CRIT"].strip().strip('"').upper()
    require(window_mode in {"YES", "NO", "TRUE", "FALSE"}, "WINDOW_CAUCHY_CRIT invalid")
    window_enabled = window_mode in {"YES", "TRUE"}
    require(report["window_cauchy_enabled"] == ("YES" if window_enabled else "NO"),
            "window convergence enablement mismatch")
    window_fields: list[str] = []
    window_start = convergence_start = window_elements = None
    window_eps = window_function = ""
    if window_enabled:
        window_fields = field_list(config_values["CONV_WINDOW_FIELD"], "CONV_WINDOW_FIELD")
        window_start = int(config_values["WINDOW_START_ITER"])
        convergence_start = int(config_values["CONV_WINDOW_STARTITER"])
        window_elements = int(config_values["CONV_WINDOW_CAUCHY_ELEMS"])
        window_eps = config_values["CONV_WINDOW_CAUCHY_EPS"].strip()
        window_function = config_values["WINDOW_FUNCTION"].strip().strip('"').upper()
        require(window_start >= 0 and convergence_start >= 0 and window_elements > 0,
                "window iteration/elements are invalid")
        require(math.isfinite(float(window_eps)) and float(window_eps) > 0.0,
                "window epsilon is invalid")
        require(window_function in {"SQUARE", "HANN", "HANN_SQUARE", "BUMP"},
                "window function is invalid")
        restart = optional_int(config_values, "RESTART_ITER")
        require(restart is None or window_start >= restart, "WINDOW_START_ITER precedes RESTART_ITER")
    require(report["window_convergence_fields"].split(";") if report["window_convergence_fields"] else [] == window_fields,
            "window field list mismatch")
    require(report["window_start_iter"] == ("" if window_start is None else str(window_start)),
            "window start mismatch")
    require(report["window_convergence_start_iter"] == ("" if convergence_start is None else str(convergence_start)),
            "window convergence start mismatch")
    require(report["window_cauchy_elements"] == ("" if window_elements is None else str(window_elements)),
            "window Cauchy element count mismatch")
    require(report["window_cauchy_epsilon"] == window_eps, "window Cauchy epsilon mismatch")
    require(report["window_function"] == window_function, "window function mismatch")

    selectors = field_list(config_values["HISTORY_OUTPUT"], "HISTORY_OUTPUT")
    require(report["history_output_selectors"].split(";") == selectors, "HISTORY_OUTPUT selector mismatch")
    discovered = independent_inventory(stdout.decode("utf-8"), stderr.decode("utf-8"))
    reported_discovered = report["available_history_fields"].split(";")
    required = report["required_history_fields"].split(";")
    require(discovered == reported_discovered, "available fields differ from independent inventory")
    active_convergence_fields = steady_fields + window_fields
    require(all(item in discovered for item in active_convergence_fields),
            "configured steady/window field missing from exact dry-run inventory")
    require(all(item in discovered for item in required), "required history field absent from inventory")

    buffer = bytearray()
    for value in [
        SCHEMA, report["run_id"], report["solver_id"], report["runner_receipt_sha256"],
        report["executable_sha256"], report["configuration_path"], config_sha,
        report["mesh_path"], mesh_sha, report["solver_family"], steady_fields[0],
        stdout_sha, stderr_sha, report["status"], EVIDENCE,
    ]:
        add_string(buffer, value)
    add_u64(buffer, len(config))
    add_u64(buffer, len(mesh))
    add_u64(buffer, len(steady_fields))
    for item in steady_fields:
        add_string(buffer, item)
    add_optional_u64(buffer, steady_start)
    add_optional_string(buffer, residual_min)
    add_optional_string(buffer, steady_eps)
    add_optional_u64(buffer, steady_elems)
    if window_enabled:
        buffer.append(1)
        add_u64(buffer, len(window_fields))
        for item in window_fields:
            add_string(buffer, item)
        add_u64(buffer, window_start)
        add_u64(buffer, convergence_start)
        add_u64(buffer, window_elements)
        add_string(buffer, window_eps)
        add_string(buffer, window_function)
    else:
        buffer.append(0)
    add_u64(buffer, len(selectors))
    for item in selectors:
        add_string(buffer, item)
    add_u64(buffer, len(discovered))
    for item in discovered:
        add_string(buffer, item)
    add_u64(buffer, len(required))
    for item in required:
        add_string(buffer, item)
    digest = hashlib.sha256(buffer).hexdigest()
    require(report["preflight_sha256"] == digest, "canonical preflight digest mismatch")
    print(
        "independent_su2_field_preflight_oracle=pass "
        f"available_fields={len(discovered)} required_fields={len(required)} "
        f"steady_fields={len(steady_fields)} window_enabled={window_enabled} receipt_bound=true"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent SU2 field preflight oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
