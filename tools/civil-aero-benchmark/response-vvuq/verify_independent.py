#!/usr/bin/env python3
"""Independent standard-library oracle for AER-01 response-vector VVUQ.

This implementation does not import the Rust crate. It independently rebuilds
identity/normalization hashes, reorders covariance matrices, validates
provenance and calibration envelopes, and computes the covariance-weighted
distance and marginal diagnostics.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "data" / "known-answer-response-vector-v0.1.json"


class AuditError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def canonical_sha256(value: Any) -> str:
    encoded = json.dumps(value, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def valid_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(ch in "0123456789abcdef" for ch in value)
    )


def require_sha(value: Any, label: str) -> None:
    require(valid_sha256(value), f"{label}: invalid SHA-256")


def require_nonempty(value: Any, label: str) -> None:
    require(isinstance(value, str) and value.strip(), f"{label}: must be non-empty")


def finite(value: Any, label: str) -> float:
    require(
        isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(float(value)),
        f"{label}: must be finite",
    )
    return float(value)


def channel_order(request: dict[str, Any]) -> list[dict[str, Any]]:
    channels = request.get("channels")
    require(isinstance(channels, list) and channels, "response vector has no channels")
    ordered = sorted(channels, key=lambda item: item.get("id", ""))
    seen: set[str] = set()
    for channel in ordered:
        for key in ("id", "quantity", "units", "coordinate_frame", "sample_id"):
            require_nonempty(channel.get(key), f"channel.{key}")
        channel_id = channel["id"]
        require(channel_id not in seen, f"duplicate channel ID {channel_id}")
        seen.add(channel_id)
        scale = finite(channel.get("normalization_scale"), f"{channel_id}.normalization_scale")
        require(scale > 0.0, f"{channel_id}: normalization scale must be positive")
        finite(channel.get("reference_value"), f"{channel_id}.reference_value")
        finite(channel.get("simulation_value"), f"{channel_id}.simulation_value")
    return ordered


def identity_hash(request: dict[str, Any]) -> str:
    identity = request.get("identity")
    require(isinstance(identity, dict), "identity missing")
    for key in (
        "configuration_sha256",
        "geometry_sha256",
        "mesh_sha256",
        "loading_state_sha256",
        "environment_sha256",
        "solver_build_sha256",
        "claim_scope_sha256",
    ):
        require_sha(identity.get(key), f"identity.{key}")
    require_nonempty(identity.get("mission_phase_id"), "identity.mission_phase_id")
    return canonical_sha256(identity)


def basis_hash(channels: list[dict[str, Any]]) -> str:
    basis = []
    for channel in channels:
        basis.append({
            "id": channel["id"],
            "quantity": channel["quantity"],
            "units": channel["units"],
            "coordinate_frame": channel["coordinate_frame"],
            "sample_id": channel["sample_id"],
            "normalization_scale": float(channel["normalization_scale"]),
        })
    return canonical_sha256(basis)


def validate_matrix(matrix: Any, n: int, label: str) -> list[list[float]]:
    require(isinstance(matrix, list) and len(matrix) == n, f"{label}: row count mismatch")
    result = []
    for row_index, row in enumerate(matrix):
        require(isinstance(row, list) and len(row) == n, f"{label}: column count mismatch")
        result.append([finite(value, f"{label}[{row_index}]") for value in row])
    scale = max(1.0, max((abs(value) for row in result for value in row), default=0.0))
    for i in range(n):
        for j in range(i):
            require(abs(result[i][j] - result[j][i]) <= 1e-12 * scale,
                    f"{label}: asymmetric covariance")
    return result


def reorder_covariance(block: dict[str, Any], ids: list[str], label: str) -> list[list[float]]:
    channel_ids = block.get("channel_ids")
    require(isinstance(channel_ids, list) and len(channel_ids) == len(ids),
            f"{label}: channel ID count mismatch")
    require(all(isinstance(value, str) and value for value in channel_ids),
            f"{label}: invalid channel ID")
    require(len(channel_ids) == len(set(channel_ids)), f"{label}: duplicate matrix channel IDs")
    require(set(channel_ids) == set(ids), f"{label}: channel set mismatch")
    matrix = validate_matrix(block.get("matrix"), len(ids), label)
    positions = {channel_id: index for index, channel_id in enumerate(channel_ids)}
    return [[matrix[positions[row]][positions[column]] for column in ids] for row in ids]


def cholesky(matrix: list[list[float]], *, semidefinite: bool, label: str) -> list[list[float]]:
    n = len(matrix)
    scale = max(1.0, max(abs(value) for row in matrix for value in row))
    tolerance = 1e-12 * scale
    lower = [[0.0] * n for _ in range(n)]
    for i in range(n):
        for j in range(i + 1):
            residual = matrix[i][j] - sum(lower[i][k] * lower[j][k] for k in range(j))
            if i == j:
                if semidefinite:
                    require(residual >= -tolerance,
                            f"{label}: covariance component is not positive semidefinite")
                    lower[i][j] = math.sqrt(max(residual, 0.0))
                else:
                    require(residual > 1e-12 * max(matrix[k][k] for k in range(n)),
                            f"{label}: combined covariance is singular or ill-conditioned")
                    lower[i][j] = math.sqrt(residual)
            else:
                if lower[j][j] > math.sqrt(tolerance):
                    lower[i][j] = residual / lower[j][j]
                else:
                    require(abs(residual) <= tolerance,
                            f"{label}: inconsistent zero covariance pivot")
    return lower


def forward_solve(lower: list[list[float]], rhs: list[float]) -> list[float]:
    result = [0.0] * len(rhs)
    for i in range(len(rhs)):
        value = rhs[i] - sum(lower[i][j] * result[j] for j in range(i))
        result[i] = value / lower[i][i]
    return result


def backward_solve_transpose(lower: list[list[float]], rhs: list[float]) -> list[float]:
    result = [0.0] * len(rhs)
    for i in reversed(range(len(rhs))):
        value = rhs[i] - sum(lower[j][i] * result[j] for j in range(i + 1, len(rhs)))
        result[i] = value / lower[i][i]
    return result


def infinity_norm(matrix: list[list[float]]) -> float:
    return max(sum(abs(value) for value in row) for row in matrix)


def condition_estimate(matrix: list[list[float]], lower: list[list[float]]) -> float:
    n = len(matrix)
    inverse = [[0.0] * n for _ in range(n)]
    for column in range(n):
        unit = [0.0] * n
        unit[column] = 1.0
        solved = backward_solve_transpose(lower, forward_solve(lower, unit))
        for row in range(n):
            inverse[row][column] = solved[row]
    return infinity_norm(matrix) * infinity_norm(inverse)


def add_matrices(*matrices: list[list[float]]) -> list[list[float]]:
    n = len(matrices[0])
    return [
        [sum(matrix[i][j] for matrix in matrices) for j in range(n)]
        for i in range(n)
    ]


def validate_block(
    block: dict[str, Any],
    ids: list[str],
    state_sha: str,
    basis_sha: str,
    held_out_sha: str,
    label: str,
    *,
    require_envelope: bool,
) -> list[list[float]]:
    require(isinstance(block, dict), f"{label}: block missing")
    require_nonempty(block.get("source_id"), f"{label}.source_id")
    for key in (
        "artifact_sha256",
        "estimator_build_sha256",
        "calibration_dataset_sha256",
        "calibration_envelope_sha256",
        "state_identity_sha256",
        "normalization_basis_sha256",
    ):
        require_sha(block.get(key), f"{label}.{key}")
    require(block["calibration_dataset_sha256"] != held_out_sha,
            f"{label}: calibration and validation data overlap")
    require(block["state_identity_sha256"] == state_sha, f"{label}: stale state binding")
    require(block["normalization_basis_sha256"] == basis_sha, f"{label}: wrong normalization basis")
    matrix = reorder_covariance(block, ids, label)

    if require_envelope:
        bounds = block.get("max_diagonal_variance")
        require(isinstance(bounds, list) and len(bounds) == len(ids),
                f"{label}: calibration variance envelope missing or wrong size")
        for index, bound in enumerate(bounds):
            cap = finite(bound, f"{label}.max_diagonal_variance[{index}]")
            require(cap > 0.0, f"{label}: calibration variance cap must be positive")
            require(matrix[index][index] <= cap * (1.0 + 1e-12),
                    f"{label}: variance exceeds its calibration envelope")
        cholesky(matrix, semidefinite=True, label=label)
    else:
        require(block.get("max_diagonal_variance") is None,
                f"{label}: cross covariance must not declare a marginal variance envelope")
    return matrix


def evaluate(request: dict[str, Any]) -> dict[str, Any]:
    require(request.get("schema_version") == "aero-response-vvuq/0.1.0",
            "unsupported schema version")
    require_nonempty(request.get("response_family_id"), "response family ID")
    held_out = request.get("held_out_validation_dataset_sha256")
    require_sha(held_out, "held-out validation dataset digest")
    if request.get("model_discrepancy_evidence_sha256") is not None:
        require_sha(request["model_discrepancy_evidence_sha256"], "model discrepancy evidence")

    state_sha = identity_hash(request)
    channels = channel_order(request)
    ids = [channel["id"] for channel in channels]
    basis_sha = basis_hash(channels)
    uncertainty = request.get("uncertainty")
    require(isinstance(uncertainty, dict), "uncertainty contract missing")
    independent = uncertainty.get("independent_components")
    require(isinstance(independent, bool), "independence declaration missing")

    source_blocks = [
        ("experimental", uncertainty.get("experimental")),
        ("numerical", uncertainty.get("numerical")),
        ("input", uncertainty.get("input")),
    ]
    source_ids = set()
    matrices = []
    receipts = []
    for label, block in source_blocks:
        matrix = validate_block(block, ids, state_sha, basis_sha, held_out, label, require_envelope=True)
        require(block["source_id"] not in source_ids, "covariance source IDs must be distinct")
        source_ids.add(block["source_id"])
        matrices.append(matrix)
        receipts.append({
            "source_id": block["source_id"],
            "artifact_sha256": block["artifact_sha256"],
            "estimator_build_sha256": block["estimator_build_sha256"],
            "calibration_dataset_sha256": block["calibration_dataset_sha256"],
            "calibration_envelope_sha256": block["calibration_envelope_sha256"],
        })

    cross = uncertainty.get("cross_source")
    if independent:
        require(cross is None, "cross covariance supplied despite independence declaration")
    else:
        require(isinstance(cross, dict), "cross covariance required for dependent uncertainty sources")
        cross_matrix = validate_block(
            cross, ids, state_sha, basis_sha, held_out, "cross-source", require_envelope=False
        )
        require(cross["source_id"] not in source_ids, "cross covariance source ID must be distinct")
        source_ids.add(cross["source_id"])
        matrices.append(cross_matrix)

    total = add_matrices(*matrices)
    lower = cholesky(total, semidefinite=False, label="combined covariance")
    condition = condition_estimate(total, lower)
    require(math.isfinite(condition) and condition <= 1.0e12,
            "combined covariance condition estimate exceeds limit")

    residuals = [
        (float(channel["simulation_value"]) - float(channel["reference_value"]))
        / float(channel["normalization_scale"])
        for channel in channels
    ]
    whitened = forward_solve(lower, residuals)
    d2 = sum(value * value for value in whitened)
    diagnostics = []
    maximum_z = 0.0
    for index, channel in enumerate(channels):
        variance = total[index][index]
        require(variance > 0.0, "combined channel variance is not positive")
        standard_deviation = math.sqrt(variance)
        marginal_z = residuals[index] / standard_deviation
        maximum_z = max(maximum_z, abs(marginal_z))
        diagnostics.append({
            "id": channel["id"],
            "quantity": channel["quantity"],
            "units": channel["units"],
            "coordinate_frame": channel["coordinate_frame"],
            "sample_id": channel["sample_id"],
            "reference_value": float(channel["reference_value"]),
            "simulation_value": float(channel["simulation_value"]),
            "physical_residual": float(channel["simulation_value"]) - float(channel["reference_value"]),
            "normalized_residual": residuals[index],
            "combined_standard_deviation": standard_deviation,
            "marginal_standardized_residual": marginal_z,
        })

    limit = request.get("decision_limit")
    if limit is None:
        status = "DIAGNOSTIC_ONLY"
        max_d2 = max_z = protocol = None
    else:
        max_d2 = finite(limit.get("max_d2"), "max_d2")
        max_z = finite(limit.get("max_abs_marginal_z"), "max_abs_marginal_z")
        require(max_d2 >= 0.0 and max_z >= 0.0, "decision limits must be non-negative")
        require_sha(limit.get("protocol_sha256"), "decision protocol")
        require_sha(limit.get("claim_scope_sha256"), "decision claim scope")
        require(limit["claim_scope_sha256"] == request["identity"]["claim_scope_sha256"],
                "decision limit claim scope mismatch")
        protocol = limit["protocol_sha256"]
        status = "EXCEEDS_DECLARED_LIMIT" if d2 > max_d2 or maximum_z > max_z else "WITHIN_DECLARED_LIMIT"

    diag_values = [total[i][i] for i in range(len(ids))]
    min_pivot = max_pivot = 0.0
    # Use the same Cholesky diagonal pivot definitions as the Rust result.
    for i in range(len(ids)):
        # Recover the positive pivot as the variance remaining after earlier columns.
        residual = total[i][i] - sum(lower[i][k] ** 2 for k in range(i))
        min_pivot = residual if i == 0 else min(min_pivot, residual)
        max_pivot = max(max_pivot, residual)

    return {
        "schema_version": "aero-response-vvuq-result/0.1.0",
        "response_family_id": request["response_family_id"],
        "state_identity_sha256": state_sha,
        "normalization_basis_sha256": basis_sha,
        "held_out_validation_dataset_sha256": held_out,
        "channel_diagnostics": diagnostics,
        "covariance_weighted_d2": d2,
        "covariance_condition_infinity_estimate": condition,
        "minimum_cholesky_pivot": min_pivot,
        "maximum_cholesky_pivot": max_pivot,
        "maximum_absolute_marginal_z": maximum_z,
        "uncertainty_sources": receipts,
        "cross_source_covariance_source_id": cross.get("source_id") if cross is not None else None,
        "decision_status": status,
        "decision_limit_d2": max_d2,
        "decision_limit_max_abs_marginal_z": max_z,
        "decision_protocol_sha256": protocol,
        "model_discrepancy_evidence_sha256": request.get("model_discrepancy_evidence_sha256"),
        "claim_ceiling": "multivariate_response_discrepancy_only",
    }


def expect_rejected(label: str, action) -> None:
    try:
        action()
    except (AuditError, KeyError, TypeError, IndexError):
        print(f"negative_control=pass case={label}")
        return
    raise AuditError(f"negative control unexpectedly passed: {label}")


def compare_rust(actual: dict[str, Any], expected: dict[str, Any]) -> None:
    scalar_fields = (
        "schema_version",
        "response_family_id",
        "state_identity_sha256",
        "normalization_basis_sha256",
        "held_out_validation_dataset_sha256",
        "decision_status",
        "decision_protocol_sha256",
        "model_discrepancy_evidence_sha256",
        "claim_ceiling",
    )
    for key in scalar_fields:
        require(actual.get(key) == expected.get(key), f"Rust/Python mismatch for {key}")
    for key in (
        "covariance_weighted_d2",
        "covariance_condition_infinity_estimate",
        "minimum_cholesky_pivot",
        "maximum_cholesky_pivot",
        "maximum_absolute_marginal_z",
    ):
        require(math.isclose(float(actual[key]), float(expected[key]), rel_tol=1e-10, abs_tol=1e-12),
                f"Rust/Python numeric mismatch for {key}")
    require(actual.get("channel_diagnostics") == expected.get("channel_diagnostics"),
            "Rust/Python channel diagnostic mismatch")
    require(actual.get("uncertainty_sources") == expected.get("uncertainty_sources"),
            "Rust/Python uncertainty source receipt mismatch")


def hostile_controls(request: dict[str, Any]) -> None:
    bad = copy.deepcopy(request)
    bad["channels"].pop()
    expect_rejected("missing_channel", lambda: evaluate(bad))

    bad = copy.deepcopy(request)
    bad["channels"][1]["id"] = bad["channels"][0]["id"]
    expect_rejected("duplicate_channel", lambda: evaluate(bad))

    bad = copy.deepcopy(request)
    bad["channels"][0]["units"] = "kN"
    expect_rejected("wrong_unit_normalization_basis", lambda: evaluate(bad))

    bad = copy.deepcopy(request)
    bad["identity"]["geometry_sha256"] = "f" * 64
    expect_rejected("stale_geometry_identity", lambda: evaluate(bad))

    bad = copy.deepcopy(request)
    bad["uncertainty"]["experimental"]["matrix"][0][1] += 0.01
    expect_rejected("asymmetric_covariance", lambda: evaluate(bad))

    bad = copy.deepcopy(request)
    bad["uncertainty"]["experimental"]["matrix"] = [
        [0.04, 0.20, 0.0],
        [0.20, 0.09, 0.0],
        [0.0, 0.0, 0.04],
    ]
    expect_rejected("indefinite_covariance_component", lambda: evaluate(bad))

    bad = copy.deepcopy(request)
    bad["uncertainty"]["experimental"]["matrix"][1][1] = 0.5
    expect_rejected("variance_inflation_beyond_calibration_envelope", lambda: evaluate(bad))

    bad = copy.deepcopy(request)
    bad["uncertainty"]["input"]["calibration_dataset_sha256"] = bad["held_out_validation_dataset_sha256"]
    expect_rejected("calibration_validation_data_overlap", lambda: evaluate(bad))

    bad = copy.deepcopy(request)
    bad["decision_limit"]["claim_scope_sha256"] = "0" * 64
    expect_rejected("wrong_decision_claim_scope", lambda: evaluate(bad))

    bad = copy.deepcopy(request)
    bad["uncertainty"]["independent_components"] = False
    expect_rejected("dependent_components_without_cross_covariance", lambda: evaluate(bad))

    bad = copy.deepcopy(request)
    bad["uncertainty"]["experimental"]["max_diagonal_variance"] = [0.001, 0.001, 0.001]
    expect_rejected("unqualified_covariance_envelope", lambda: evaluate(bad))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path)
    args = parser.parse_args()
    try:
        request = json.loads(FIXTURE.read_text(encoding="utf-8"))
        expected = evaluate(request)
        require(math.isclose(expected["covariance_weighted_d2"], 18.493035490698507, rel_tol=1e-10),
                "frozen correlated-covariance known answer changed")
        require(expected["decision_status"] == "WITHIN_DECLARED_LIMIT",
                "known-answer decision status changed")

        diagnostic = copy.deepcopy(request)
        diagnostic["decision_limit"] = None
        require(evaluate(diagnostic)["decision_status"] == "DIAGNOSTIC_ONLY",
                "missing decision limit must remain diagnostic-only")

        local_outlier = copy.deepcopy(request)
        local_outlier["decision_limit"]["max_d2"] = 1_000_000.0
        local_outlier["decision_limit"]["max_abs_marginal_z"] = 2.0
        require(evaluate(local_outlier)["decision_status"] == "EXCEEDS_DECLARED_LIMIT",
                "local component gate failed to preserve outlier")

        hostile_controls(request)
        if args.rust_output is not None:
            actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
            compare_rust(actual, expected)
            boundary = "rust_python_boundary=pass"
        else:
            boundary = "rust_python_boundary=not_run"

        print(
            "aero_response_vvuq_oracle=pass "
            f"channels={len(expected['channel_diagnostics'])} "
            f"d2={expected['covariance_weighted_d2']:.12f} "
            f"max_abs_z={expected['maximum_absolute_marginal_z']:.12f} "
            f"{boundary}"
        )
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"aero_response_vvuq_oracle_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
