#!/usr/bin/env python3
"""Independent stdlib replay for the normalized BOCoDe task catalog."""
from __future__ import annotations

import csv
import hashlib
import struct
import sys
from pathlib import Path

SCHEMA = "bocode-task-catalog/0.1.0"
OBJECTIVE_DIRECTION = "maximize"
CONSTRAINT_CONVENTION = "g(x)<=0_is_feasible"
EVIDENCE_LEVEL = (
    "external_benchmark_metadata_import_only_task_execution_and_scientific_validity_unverified"
)
HEADER = (
    "problem_id,application,problem_class,dimension,num_objectives,num_constraints,"
    "variable_type_class,benchmark_set_member,bounds_state,bounds_sha256,"
    "variable_types_state,variable_types_sha256,source_reference_state,source_reference,"
    "metadata_sha256,required_extra"
).split(",")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def append_u64(buffer: bytearray, value: int) -> None:
    require(0 <= value < (1 << 64), "canonical integer outside u64")
    buffer.extend(struct.pack("<Q", value))


def append_string(buffer: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    append_u64(buffer, len(encoded))
    buffer.extend(encoded)


def valid_hex(value: str, length: int) -> bool:
    return len(value) == length and all(ch in "0123456789abcdefABCDEF" for ch in value)


def main(path: Path, source_commit: str, package_version: str,
         source_catalog_sha256: str, expected_import_sha256: str) -> int:
    require(valid_hex(source_commit, 40), "source commit must be full 40-character hexadecimal")
    require(bool(package_version.strip()), "package version must be non-empty")
    require(valid_hex(source_catalog_sha256, 64), "source catalog digest must be SHA-256")
    require(valid_hex(expected_import_sha256, 64), "expected import digest must be SHA-256")

    with path.open("r", encoding="utf-8", newline="") as stream:
        reader = csv.DictReader(stream)
        require(reader.fieldnames == HEADER, "BOCoDe catalog CSV header mismatch")
        rows = list(reader)
    require(rows, "BOCoDe catalog must contain at least one task")

    supported_domains = {"engineering", "hpo", "synthetic", "extended"}
    supported_classes = {
        "so_unconstrained", "so_constrained", "mo_unconstrained",
        "mo_constrained", "so_mixed_variable",
        "so_mixed_variable_constrained_extension", "mo_mixed_variable_extension",
        "mo_mixed_variable_constrained_extension",
    }
    previous_id = None
    seen = set()
    tasks = []
    engineering_count = 0
    benchmark_count = 0

    for line_number, row in enumerate(rows, start=2):
        problem_id = row["problem_id"]
        require(problem_id.strip() != "", f"line {line_number}: problem_id required")
        require(problem_id not in seen, f"line {line_number}: duplicate problem_id")
        require(previous_id is None or previous_id < problem_id,
                f"line {line_number}: problem IDs must be strictly sorted")
        seen.add(problem_id)
        previous_id = problem_id

        domain = row["application"]
        problem_class = row["problem_class"]
        variable_class = row["variable_type_class"]
        require(domain in supported_domains, f"line {line_number}: unknown application domain")
        require(problem_class in supported_classes, f"line {line_number}: unknown problem class")
        require(variable_class in ("continuous", "mixed"),
                f"line {line_number}: unknown variable type class")
        try:
            dimension = int(row["dimension"])
            objectives = int(row["num_objectives"])
            constraints = int(row["num_constraints"])
        except ValueError as error:
            raise ValueError(f"line {line_number}: counts must be integers") from error
        require(dimension > 0 and objectives > 0 and constraints >= 0,
                f"line {line_number}: invalid dimension/objective/constraint count")
        member = row["benchmark_set_member"]
        require(member in ("true", "false"), f"line {line_number}: invalid benchmark_set_member")
        for name in ("bounds_sha256", "variable_types_sha256", "metadata_sha256"):
            require(valid_hex(row[name], 64), f"line {line_number}: {name} must be SHA-256")
        require(row["bounds_state"] in ("present", "unavailable"),
                f"line {line_number}: invalid bounds_state")
        require(row["variable_types_state"] in ("present", "unavailable"),
                f"line {line_number}: invalid variable_types_state")
        source_state = row["source_reference_state"]
        require(source_state in ("present", "missing"),
                f"line {line_number}: invalid source_reference_state")
        if source_state == "present":
            require(row["source_reference"].strip() != "",
                    f"line {line_number}: present source citation must be non-empty")
        else:
            require(row["source_reference"] == "",
                    f"line {line_number}: missing source state must have an empty citation")
        require(row["required_extra"].strip() != "",
                f"line {line_number}: required_extra must be explicit (use none)")

        expected = {
            "so_unconstrained": objectives == 1 and constraints == 0 and variable_class == "continuous",
            "so_constrained": objectives == 1 and constraints > 0 and variable_class == "continuous",
            "mo_unconstrained": objectives > 1 and constraints == 0 and variable_class == "continuous",
            "mo_constrained": objectives > 1 and constraints > 0 and variable_class == "continuous",
            "so_mixed_variable": objectives == 1 and constraints == 0 and variable_class == "mixed",
            "so_mixed_variable_constrained_extension": objectives == 1 and constraints > 0 and variable_class == "mixed",
            "mo_mixed_variable_extension": objectives > 1 and constraints == 0 and variable_class == "mixed",
            "mo_mixed_variable_constrained_extension": objectives > 1 and constraints > 0 and variable_class == "mixed",
        }
        require(expected[problem_class],
                f"line {line_number}: problem class contradicts objective/constraint/variable metadata")

        normalized = {
            "problem_id": problem_id,
            "application": domain,
            "problem_class": problem_class,
            "dimension": dimension,
            "num_objectives": objectives,
            "num_constraints": constraints,
            "variable_type_class": variable_class,
            "benchmark_set_member": member == "true",
            "bounds_state": row["bounds_state"],
            "bounds_sha256": row["bounds_sha256"],
            "variable_types_state": row["variable_types_state"],
            "variable_types_sha256": row["variable_types_sha256"],
            "source_reference_state": source_state,
            "source_reference": row["source_reference"],
            "metadata_sha256": row["metadata_sha256"],
            "required_extra": row["required_extra"],
        }
        tasks.append(normalized)
        engineering_count += domain == "engineering" and member == "true"
        benchmark_count += member == "true"

    buffer = bytearray()
    for value in (
        SCHEMA, source_commit, package_version, source_catalog_sha256,
        OBJECTIVE_DIRECTION, CONSTRAINT_CONVENTION, EVIDENCE_LEVEL,
    ):
        append_string(buffer, value)
    append_u64(buffer, len(tasks))
    for task in tasks:
        for name in (
            "problem_id", "application", "problem_class", "variable_type_class",
            "bounds_state", "bounds_sha256", "variable_types_state",
            "variable_types_sha256", "source_reference_state", "source_reference",
            "metadata_sha256", "required_extra",
        ):
            append_string(buffer, str(task[name]))
        append_u64(buffer, task["dimension"])
        append_u64(buffer, task["num_objectives"])
        append_u64(buffer, task["num_constraints"])
        buffer.append(1 if task["benchmark_set_member"] else 0)

    actual = hashlib.sha256(buffer).hexdigest()
    require(actual == expected_import_sha256, "BOCoDe import digest mismatch")
    print(
        "independent_bocode_catalog_oracle=pass "
        f"tasks={len(tasks)} benchmark_set={benchmark_count} engineering={engineering_count} "
        f"objective_direction={OBJECTIVE_DIRECTION} constraints={CONSTRAINT_CONVENTION}"
    )
    return 0


if __name__ == "__main__":
    try:
        if len(sys.argv) != 6:
            raise ValueError(
                "usage: verify-bocode-catalog-independent.py <catalog.csv> "
                "<source-commit> <package-version> <source-catalog-sha256> <import-sha256>"
            )
        raise SystemExit(main(
            Path(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]
        ))
    except Exception as error:
        print(f"independent BOCoDe catalog oracle failed: {error}", file=sys.stderr)
        raise
