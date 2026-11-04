#!/usr/bin/env python3
"""Independent stdlib replay for the covariance-aware joint validation CSV."""
from __future__ import annotations

import csv
import hashlib
import math
import struct
import sys
from collections import defaultdict
from pathlib import Path

SCHEMA = "aero-multivariate-validation/0.1.0"
EVIDENCE_LEVEL = (
    "covariance_aware_joint_discrepancy_diagnostic_only_no_distributional_coverage_or_acceptance_claim"
)
HEADER = (
    "record_type,schema,study_id,model_id,observation_id,geometry_family,flow_regime,"
    "dataset_split,domain_status,uncertainty_source_reference,uncertainty_source_sha256,"
    "uncertainty_model_rationale,output_index,quantity,unit,predicted_value,observed_value,"
    "standard_uncertainty,correlation_row,standardized_residual,whitened_residual,"
    "mahalanobis_squared,mahalanobis_distance,in_domain_observation_count,"
    "out_of_domain_observation_count,mean_mahalanobis_squared,maximum_mahalanobis_squared,"
    "observation_sha256,study_sha256,report_sha256"
).split(",")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def finite_float(row: dict[str, str], name: str) -> float:
    value = float(row[name])
    require(math.isfinite(value), f"{name} must be finite")
    return value


def parse_correlation_row(value: str) -> list[float]:
    require(bool(value), "correlation row must not be empty")
    result = [float(part) for part in value.split(";")]
    require(all(math.isfinite(x) for x in result), "correlation row must be finite")
    return result


def append_u64(buffer: bytearray, value: int) -> None:
    require(0 <= value < (1 << 64), "canonical length outside u64")
    buffer.extend(struct.pack("<Q", value))


def append_string(buffer: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    append_u64(buffer, len(encoded))
    buffer.extend(encoded)


def append_f64(buffer: bytearray, value: float) -> None:
    buffer.extend(struct.pack("<d", value))


def sha256(buffer: bytes) -> str:
    return hashlib.sha256(buffer).hexdigest()


def close(left: float, right: float) -> bool:
    return abs(left - right) <= 1.0e-12 * max(abs(left), abs(right), 1.0)


def vectors_close(left: list[float], right: list[float]) -> bool:
    return len(left) == len(right) and all(close(a, b) for a, b in zip(left, right))


def cholesky_lower(matrix: list[list[float]]) -> list[list[float]]:
    n = len(matrix)
    require(n > 0 and all(len(row) == n for row in matrix),
            "correlation matrix must be non-empty and square")
    for i in range(n):
        for j in range(n):
            require(math.isfinite(matrix[i][j]), "correlation matrix must be finite")
            require(abs(matrix[i][j] - matrix[j][i]) <= 1.0e-10,
                    "correlation matrix must be symmetric")
        require(abs(matrix[i][i] - 1.0) <= 1.0e-10,
                "correlation matrix diagonal entries must equal one")
    lower = [[0.0] * n for _ in range(n)]
    for i in range(n):
        for j in range(i + 1):
            value = matrix[i][j]
            for k in range(j):
                value -= lower[i][k] * lower[j][k]
            if i == j:
                require(math.isfinite(value) and value > 1.0e-12,
                        "correlation matrix must be strictly positive definite with stable pivots")
                lower[i][j] = math.sqrt(value)
            else:
                require(lower[j][j] > 0.0, "invalid Cholesky pivot")
                lower[i][j] = value / lower[j][j]
    return lower


def solve_lower(lower: list[list[float]], rhs: list[float]) -> list[float]:
    require(len(lower) == len(rhs), "whitening vector dimension mismatch")
    solution: list[float] = []
    for i in range(len(rhs)):
        value = rhs[i]
        for j in range(i):
            value -= lower[i][j] * solution[j]
        value /= lower[i][i]
        require(math.isfinite(value), "whitened residual must be finite")
        solution.append(value)
    return solution


def observation_digest(study_id: str, model_id: str, meta: dict[str, str],
                       outputs: list[dict[str, object]], matrix: list[list[float]]) -> str:
    b = bytearray()
    for item in (
        SCHEMA, study_id, model_id, meta["observation_id"], meta["geometry_family"],
        meta["flow_regime"], meta["dataset_split"], meta["domain_status"],
        meta["uncertainty_source_reference"], meta["uncertainty_source_sha256"],
        meta["uncertainty_model_rationale"],
    ):
        append_string(b, item)
    append_u64(b, len(outputs))
    for output in outputs:
        append_string(b, str(output["quantity"]))
        append_string(b, str(output["unit"]))
        append_f64(b, float(output["predicted_value"]))
        append_f64(b, float(output["observed_value"]))
        append_f64(b, float(output["standard_uncertainty"]))
    append_u64(b, len(matrix))
    for row in matrix:
        append_u64(b, len(row))
        for value in row:
            append_f64(b, value)
    return sha256(bytes(b))


def main(path: Path) -> int:
    with path.open("r", encoding="utf-8", newline="") as stream:
        rows = list(csv.DictReader(stream))
    require(rows, "joint validation CSV must contain records")
    require(list(rows[0].keys()) == HEADER, "joint validation CSV header mismatch")
    require(
        all(row["record_type"] in ("study", "output") for row in rows),
        "joint validation CSV contains an unknown record type",
    )
    metadata_rows = [row for row in rows if row["record_type"] == "study"]
    output_rows = [row for row in rows if row["record_type"] == "output"]
    require(len(metadata_rows) == 1, "exactly one study metadata row is required")
    meta_row = metadata_rows[0]
    require(meta_row["schema"] == SCHEMA, "joint validation schema mismatch")
    require(bool(meta_row["study_id"].strip()) and bool(meta_row["model_id"].strip()),
            "study and model identity must be non-empty")
    require(output_rows, "joint validation needs output rows")
    require(all(row["schema"] == SCHEMA for row in output_rows), "output schema mismatch")
    study_id, model_id = meta_row["study_id"], meta_row["model_id"]

    groups: dict[str, list[dict[str, str]]] = defaultdict(list)
    for row in output_rows:
        require(row["study_id"] == study_id and row["model_id"] == model_id,
                "output row study/model identity mismatch")
        require(row["dataset_split"] == "blind_confirmation",
                "all observations must be blind_confirmation")
        require(row["domain_status"] in ("in_domain", "out_of_domain"),
                "unknown domain status")
        require(row["uncertainty_source_sha256"] and
                len(row["uncertainty_source_sha256"]) == 64 and
                all(c in "0123456789abcdef" for c in row["uncertainty_source_sha256"]),
                "uncertainty source digest must be lowercase SHA-256")
        require(row["uncertainty_source_reference"].strip() and
                row["uncertainty_model_rationale"].strip(),
                "uncertainty source and rationale are required")
        groups[row["observation_id"]].append(row)
    require("" not in groups, "observation ID must be non-empty")

    output_schema: list[tuple[str, str]] | None = None
    replayed: list[dict[str, object]] = []
    for observation_id in sorted(groups):
        group = groups[observation_id]
        group.sort(key=lambda row: int(row["output_index"]))
        require([int(row["output_index"]) for row in group] == list(range(len(group))),
                f"{observation_id}: output indexes must be contiguous from zero")
        first = group[0]
        shared_fields = (
            "geometry_family", "flow_regime", "dataset_split", "domain_status",
            "uncertainty_source_reference", "uncertainty_source_sha256",
            "uncertainty_model_rationale", "mahalanobis_squared", "mahalanobis_distance",
            "in_domain_observation_count", "out_of_domain_observation_count",
            "mean_mahalanobis_squared", "maximum_mahalanobis_squared",
            "observation_sha256", "study_sha256", "report_sha256",
        )
        for row in group[1:]:
            require(all(row[field] == first[field] for field in shared_fields),
                    f"{observation_id}: repeated observation metadata mismatch")
        outputs: list[dict[str, object]] = []
        matrix: list[list[float]] = []
        standardized_report: list[float] = []
        whitened_report: list[float] = []
        for row in group:
            quantity, unit = row["quantity"], row["unit"]
            require(quantity.strip() and unit.strip(), "output quantity and unit are required")
            predicted = finite_float(row, "predicted_value")
            observed = finite_float(row, "observed_value")
            uncertainty = finite_float(row, "standard_uncertainty")
            require(uncertainty > 0.0, "standard uncertainty must be positive")
            outputs.append({
                "quantity": quantity,
                "unit": unit,
                "predicted_value": predicted,
                "observed_value": observed,
                "standard_uncertainty": uncertainty,
            })
            matrix.append(parse_correlation_row(row["correlation_row"]))
            standardized_report.append(finite_float(row, "standardized_residual"))
            whitened_report.append(finite_float(row, "whitened_residual"))
        schema = [(str(row["quantity"]), str(row["unit"])) for row in outputs]
        require(len({name for name, _unit in schema}) == len(schema),
                "duplicate output quantity")
        require(output_schema is None or schema == output_schema,
                "output schema/order differs across observations")
        output_schema = schema
        lower = cholesky_lower(matrix)
        standardized = [
            (float(output["predicted_value"]) - float(output["observed_value"]))
            / float(output["standard_uncertainty"])
            for output in outputs
        ]
        require(all(math.isfinite(value) for value in standardized),
                "standardized residual must be finite")
        whitened = solve_lower(lower, standardized)
        squared = 0.0
        for value in whitened:
            squared += value * value
        distance = math.sqrt(squared)
        require(vectors_close(standardized_report, standardized),
                f"{observation_id}: standardized residual mismatch")
        require(vectors_close(whitened_report, whitened),
                f"{observation_id}: whitened residual mismatch")
        require(close(finite_float(first, "mahalanobis_squared"), squared),
                f"{observation_id}: Mahalanobis squared mismatch")
        require(close(finite_float(first, "mahalanobis_distance"), distance),
                f"{observation_id}: Mahalanobis distance mismatch")

        digest_meta = {field: first[field] for field in (
            "observation_id", "geometry_family", "flow_regime", "dataset_split",
            "domain_status", "uncertainty_source_reference", "uncertainty_source_sha256",
            "uncertainty_model_rationale",
        )}
        obs_sha = observation_digest(study_id, model_id, digest_meta, outputs, matrix)
        require(first["observation_sha256"] == obs_sha,
                f"{observation_id}: observation digest mismatch")
        replayed.append({
            "observation_id": observation_id,
            "domain_status": first["domain_status"],
            "observation_sha256": obs_sha,
            "standardized": standardized,
            "whitened": whitened,
            "mahalanobis_squared": squared,
            "mahalanobis_distance": distance,
        })

    in_domain = sum(row["domain_status"] == "in_domain" for row in replayed)
    out_domain = len(replayed) - in_domain
    total_squared = 0.0
    for row in replayed:
        total_squared += float(row["mahalanobis_squared"])
    mean = total_squared / len(replayed)
    maximum = max(float(row["mahalanobis_squared"]) for row in replayed)
    require(int(meta_row["in_domain_observation_count"]) == in_domain,
            "in-domain count mismatch")
    require(int(meta_row["out_of_domain_observation_count"]) == out_domain,
            "out-of-domain count mismatch")
    require(close(float(meta_row["mean_mahalanobis_squared"]), mean),
            "study mean Mahalanobis squared mismatch")
    require(close(float(meta_row["maximum_mahalanobis_squared"]), maximum),
            "study maximum Mahalanobis squared mismatch")

    b = bytearray()
    for item in (SCHEMA, study_id, model_id):
        append_string(b, item)
    append_u64(b, len(replayed))
    for row in replayed:
        append_string(b, str(row["observation_id"]))
        append_string(b, str(row["observation_sha256"]))
    study_sha = sha256(bytes(b))
    require(meta_row["study_sha256"] == study_sha, "study digest mismatch")
    require(all(row["study_sha256"] == study_sha for row in output_rows),
            "output row study digest mismatch")

    b = bytearray()
    for item in (SCHEMA, study_id, model_id, EVIDENCE_LEVEL, study_sha):
        append_string(b, item)
    append_u64(b, in_domain)
    append_u64(b, out_domain)
    append_f64(b, mean)
    append_f64(b, maximum)
    append_u64(b, len(replayed))
    for row in replayed:
        append_string(b, str(row["observation_id"]))
        append_string(b, str(row["observation_sha256"]))
        append_f64(b, float(row["mahalanobis_squared"]))
        append_f64(b, float(row["mahalanobis_distance"]))
        standardized = row["standardized"]
        whitened = row["whitened"]
        append_u64(b, len(standardized))
        for value in standardized:
            append_f64(b, float(value))
        append_u64(b, len(whitened))
        for value in whitened:
            append_f64(b, float(value))
    report_sha = sha256(bytes(b))
    require(meta_row["report_sha256"] == report_sha, "report digest mismatch")
    require(all(row["report_sha256"] == report_sha for row in output_rows),
            "output row report digest mismatch")
    print(
        "independent_joint_validation_oracle=pass "
        f"observations={len(replayed)} outputs_per_observation={len(output_schema or [])} "
        f"in_domain={in_domain} out_of_domain={out_domain} "
        "distributional_interpretation=none"
    )
    return 0


if __name__ == "__main__":
    try:
        if len(sys.argv) != 2:
            raise ValueError("usage: verify-joint-validation-independent.py <joint-validation.csv>")
        raise SystemExit(main(Path(sys.argv[1])))
    except Exception as error:
        print(f"independent joint validation oracle failed: {error}", file=sys.stderr)
        raise
