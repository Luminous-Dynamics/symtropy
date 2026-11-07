#!/usr/bin/env python3
"""Independent stdlib oracle for the finite-state EVSI demonstration."""
from __future__ import annotations

import csv
import math
import sys
from pathlib import Path

PRIOR = [0.5, 0.5]
DECISIONS = {
    "design_a": [10.0, 0.0],
    "design_b": [0.0, 10.0],
}
EXPERIMENTS = {
    "perfect_information": {
        "cost": 1.0,
        "outcomes": ["a", "b"],
        "likelihood": [[1.0, 0.0], [0.0, 1.0]],
    },
    "noisy_information": {
        "cost": 1.0,
        "outcomes": ["a_signal", "b_signal"],
        "likelihood": [[0.8, 0.2], [0.2, 0.8]],
    },
    "uninformative": {
        "cost": 0.25,
        "outcomes": ["coin_1", "coin_2"],
        "likelihood": [[0.5, 0.5], [0.5, 0.5]],
    },
    "too_expensive": {
        "cost": 8.0,
        "outcomes": ["a", "b"],
        "likelihood": [[1.0, 0.0], [0.0, 1.0]],
    },
}
BUDGET = 2.0
TOL = 2e-12


def best_decision(probabilities: list[float]) -> tuple[str, float]:
    values = {
        decision_id: sum(p * utility for p, utility in zip(probabilities, utilities, strict=True))
        for decision_id, utilities in DECISIONS.items()
    }
    best_id = min(values, key=lambda key: (-values[key], key))
    return best_id, values[best_id]


def calculate(experiment: dict[str, object]) -> tuple[float, int]:
    outcomes = experiment["outcomes"]
    likelihood = experiment["likelihood"]
    assert isinstance(outcomes, list) and isinstance(likelihood, list)
    expected_after = 0.0
    changing = 0
    prior_decision, _ = best_decision(PRIOR)
    for column, _outcome in enumerate(outcomes):
        probability = sum(PRIOR[state] * likelihood[state][column] for state in range(2))
        if probability <= 0.0:
            continue
        posterior = [
            PRIOR[state] * likelihood[state][column] / probability
            for state in range(2)
        ]
        decision, utility = best_decision(posterior)
        expected_after += probability * utility
        if decision != prior_decision:
            changing += 1
    return expected_after, changing


def close(record: dict[str, str], key: str, expected: float) -> None:
    actual = float(record[key])
    if not math.isfinite(actual) or abs(actual - expected) > TOL:
        raise ValueError(f"{record['experiment_id']} {key}: {actual} != {expected}")


def main() -> int:
    if len(sys.argv) != 2:
        raise ValueError("usage: verify-evsi-independent.py EVSI_REPORT.csv")
    path = Path(sys.argv[1])
    with path.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream))
    if len(rows) != len(EXPERIMENTS):
        raise ValueError(f"expected {len(EXPERIMENTS)} experiments, got {len(rows)}")
    actual_ids = {row["experiment_id"] for row in rows}
    if actual_ids != set(EXPERIMENTS):
        raise ValueError(f"experiment IDs differ: {actual_ids}")

    prior_decision, prior_utility = best_decision(PRIOR)
    results = []
    for row in rows:
        experiment_id = row["experiment_id"]
        model = EXPERIMENTS[experiment_id]
        expected_after, changing = calculate(model)
        gross = expected_after - prior_utility
        cost = float(model["cost"])
        affordable = cost <= BUDGET
        net = gross - cost
        if row["problem_id"] != "two-state-design-choice":
            raise ValueError("problem ID mismatch")
        if row["prior_best_decision_id"] != prior_decision:
            raise ValueError(f"{experiment_id}: prior decision mismatch")
        if row["affordable"] != str(affordable).lower():
            raise ValueError(f"{experiment_id}: affordability mismatch")
        if int(row["outcome_count"]) != len(model["outcomes"]):
            raise ValueError(f"{experiment_id}: outcome count mismatch")
        if int(row["decision_changing_outcomes"]) != changing:
            raise ValueError(f"{experiment_id}: decision-change count mismatch")
        close(row, "cost_units", cost)
        close(row, "remaining_budget_units", BUDGET)
        close(row, "prior_best_expected_utility", prior_utility)
        close(row, "expected_best_utility_after_observation", expected_after)
        close(row, "gross_evsi", gross)
        close(row, "net_evsi", net)
        if row["evidence_level"] != "caller_supplied_probabilities_not_independently_calibrated":
            raise ValueError(f"{experiment_id}: missing evidence boundary")
        results.append((experiment_id, net, affordable))

    eligible = [(experiment_id, net) for experiment_id, net, affordable in results if affordable and net > 1e-12]
    selected = min(eligible, key=lambda item: (-item[1], item[0]))[0] if eligible else None
    selected_rows = [row["experiment_id"] for row in rows if row["selected"] == "true"]
    if selected_rows != ([selected] if selected else []):
        raise ValueError(f"Rust selector chose {selected_rows}; independent oracle chose {selected}")
    print(
        "independent_evsi_oracle=pass experiments=4 "
        f"selected={selected or 'abstain'} decision_change_counts=verified"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent EVSI oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
