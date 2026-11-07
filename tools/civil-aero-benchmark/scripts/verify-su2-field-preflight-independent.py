#!/usr/bin/env python3
"""Independent receipt/config/mesh and canonical digest oracle for SU2 dry-run field preflight."""
from __future__ import annotations

import csv
import hashlib
import struct
import sys
from pathlib import Path

EVIDENCE = (
    "receipt_bound_su2_dry_run_field_discovery_and_config_presence_only_"
    "not_history_convergence_or_validation"
)
SCHEMA = "su2-field-preflight/0.1.0"


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


def main() -> int:
    if len(sys.argv) != 3:
        raise ValueError("usage: verify-su2-field-preflight-independent.py RUN_DIR REPORT_CSV")
    run = Path(sys.argv[1])
    report_rows = read_csv(Path(sys.argv[2]))
    receipt_rows = read_csv(run / "runner_receipt.csv")
    require(len(report_rows) == 1, "expected exactly one preflight report row")
    require(len(receipt_rows) == 1, "expected exactly one process receipt row")
    report = report_rows[0]
    receipt = receipt_rows[0]

    require(report["schema"] == SCHEMA, "preflight schema mismatch")
    require(report["evidence_level"] == EVIDENCE, "preflight evidence ceiling mismatch")
    require(report["status"] == "required_fields_available_and_conv_field_recognized", "preflight did not pass")
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

    config_text = config.decode("ascii")
    config_values: dict[str, str] = {}
    for raw in config_text.splitlines():
        line = raw.split("%", 1)[0].strip()
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        if key.strip() in {"SOLVER", "CONV_FIELD", "HISTORY_OUTPUT"}:
            require(key.strip() not in config_values, f"duplicate config key {key.strip()}")
            config_values[key.strip()] = value.strip()
    require(report["solver_family"] == config_values["SOLVER"], "solver family mismatch")
    configured_conv = config_values["CONV_FIELD"].strip().strip("()").strip().strip('"')
    require(report["configured_convergence_field"] == configured_conv, "CONV_FIELD mismatch")
    selectors = [item.strip().strip('"') for item in config_values["HISTORY_OUTPUT"].strip().strip("()").split(",") if item.strip()]
    require(report["history_output_selectors"].split(";") == selectors, "HISTORY_OUTPUT selector mismatch")

    discovered = independent_inventory(stdout.decode("utf-8"), stderr.decode("utf-8"))
    reported_discovered = report["available_history_fields"].split(";")
    required = report["required_history_fields"].split(";")
    require(discovered == reported_discovered, "available fields differ from independent inventory")
    require(configured_conv in discovered, "configured CONV_FIELD is not in exact dry-run inventory")
    require(all(item in discovered for item in required), "one or more required fields absent from inventory")

    buffer = bytearray()
    for value in [
        SCHEMA,
        report["run_id"],
        report["solver_id"],
        report["runner_receipt_sha256"],
        report["executable_sha256"],
        report["configuration_path"],
        config_sha,
        report["mesh_path"],
        mesh_sha,
        report["solver_family"],
        configured_conv,
        stdout_sha,
        stderr_sha,
        report["status"],
        EVIDENCE,
    ]:
        add_string(buffer, value)
    buffer.extend(struct.pack("<Q", len(config)))
    buffer.extend(struct.pack("<Q", len(mesh)))
    buffer.extend(struct.pack("<Q", len(selectors)))
    for item in selectors:
        add_string(buffer, item)
    buffer.extend(struct.pack("<Q", len(discovered)))
    for item in discovered:
        add_string(buffer, item)
    buffer.extend(struct.pack("<Q", len(required)))
    for item in required:
        add_string(buffer, item)
    digest = hashlib.sha256(buffer).hexdigest()
    require(report["preflight_sha256"] == digest, "canonical preflight digest mismatch")
    print(
        "independent_su2_field_preflight_oracle=pass "
        f"available_fields={len(discovered)} required_fields={len(required)} "
        f"conv_field={configured_conv} receipt_bound=true"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent SU2 field preflight oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
