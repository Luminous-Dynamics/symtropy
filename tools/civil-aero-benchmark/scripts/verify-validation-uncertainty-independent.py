#!/usr/bin/env python3
"""Independent stdlib oracle for the synthetic CFD validation uncertainty budget."""
from __future__ import annotations

import csv
import math
import sys
from pathlib import Path


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-validation-uncertainty-independent.py output.csv", file=sys.stderr)
        return 2
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"expected one summary row, found {len(rows)}")
    row = rows[0]
    require(row["case_id"] == "synthetic-oat15a-validation", "wrong case binding")
    require(row["quantity"] == "CD", "wrong quantity")
    require(row["admission_status"].startswith("numerically_admitted_"), "not numerically admitted")
    require(row["gci_fine_grid_id"] == "L3", "must use the finest grid")
    require(math.isclose(float(row["gci_fine_absolute"]), 1.0, abs_tol=1e-10), "GCI mismatch")
    require(math.isclose(float(row["gci_standard_uncertainty"]), 0.5, abs_tol=1e-10), "GCI conversion mismatch")
    require(math.isclose(float(row["numerical_standard_uncertainty"]), 0.525, abs_tol=1e-10), "epistemic numerical sum mismatch")
    require(math.isclose(float(row["numerical_expanded_bound"]), 1.05, abs_tol=1e-10), "numerical bound mismatch")

    # Sensitivity contributions are [2*.1, -1.5*.2] = [0.2, -0.3].
    # The covariance matrix has off-diagonal correlation 0.4.
    input_variance = 0.2**2 + (-0.3)**2 + 2.0 * 0.4 * 0.2 * (-0.3)
    input_uncertainty = math.sqrt(input_variance)
    require(math.isclose(float(row["input_standard_uncertainty"]), input_uncertainty, rel_tol=1e-10), "covariance propagation mismatch")
    require(row["input_and_experiment_are_aleatory"] == "true", "aleatory basis declaration missing")
    require(row["aleatory_basis_rationale"] == "synthetic declared aleatory sources", "aleatory rationale missing")
    require(row["input_experiment_error_correlation"] == "0.250000000000", "input/experiment correlation missing")
    aleatory = math.sqrt(input_uncertainty**2 + 0.15**2 - 2 * 0.25 * input_uncertainty * 0.15)
    require(math.isclose(float(row["aleatory_standard_uncertainty"]), aleatory, rel_tol=1e-10), "aleatory combination mismatch")
    require(math.isclose(float(row["aleatory_expanded_bound"]), 2.0 * aleatory, rel_tol=1e-10), "aleatory expanded bound mismatch")
    total_bound = 1.05 + 2.0 * aleatory
    require(math.isclose(float(row["conservative_total_bound"]), total_bound, rel_tol=1e-10), "linear conservative sum mismatch")

    error = 2.8 - 2.75
    require(math.isclose(float(row["comparison_error"]), error, abs_tol=1e-12), "comparison error mismatch")
    require(math.isclose(float(row["model_discrepancy_lower"]), error - total_bound, rel_tol=1e-10), "lower discrepancy bound mismatch")
    require(math.isclose(float(row["model_discrepancy_upper"]), error + total_bound, rel_tol=1e-10), "upper discrepancy bound mismatch")
    require("no_joint_coverage_claim" in row["evidence_level"], "joint coverage ceiling missing")
    require(row["input_component_ids"] == "Mach|Re", "input lineage missing")
    require(row["input_component_source_refs"] == "synthetic-input-uncertainty|synthetic-input-uncertainty", "input source lineage missing")
    require(row["input_correlation_matrix"] == "1.000000000000|0.400000000000;0.400000000000|1.000000000000", "correlation matrix not retained")
    print("independent_validation_uncertainty_oracle=pass u_num=0.525 u_input_covariance=verified aleatory_correlation=verified conservative_envelope=verified no_joint_coverage_claim=true")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"independent_validation_uncertainty_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
