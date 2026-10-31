#!/usr/bin/env python3
"""Independent stdlib oracle for task-clustered policy benchmark aggregation."""
from __future__ import annotations

import csv
import math
import sys
from pathlib import Path

BASELINE = [0.22, 0.38, 0.28, 0.42, 0.18, 0.35, 0.31, 0.27, 0.40, 0.21]
VOI = [0.05, 0.04, 0.03, -0.02, 0.06, -0.03, 0.02, 0.02, -0.04, 0.01]
ADJOINT = [0.12, -0.02, -0.04, 0.06, -0.01, -0.02, 0.03, -0.03, 0.09, -0.05]
JITTER = [-0.01, 0.0, 0.01]
SEED = 20_261_031
SAMPLES = 10_000
MASK = (1 << 64) - 1


def require(ok: bool, message: str) -> None:
    if not ok:
        raise ValueError(message)


def mean(values: list[float]) -> float:
    return sum(values) / len(values)


def median(values: list[float]) -> float:
    values = sorted(values)
    mid = len(values) // 2
    return (values[mid - 1] + values[mid]) / 2 if len(values) % 2 == 0 else values[mid]


def nearest_rank(values: list[float], p: float) -> float:
    values = sorted(values)
    rank = max(1, math.ceil(p * len(values)))
    return values[rank - 1]


def stable_hash(value: str) -> int:
    result = 0xCBF29CE484222325
    for byte in value.encode():
        result = ((result ^ byte) * 0x00000100000001B3) & MASK
    return result


class XorShift64:
    def __init__(self, seed: int) -> None:
        self.state = seed or 0x9E3779B97F4A7C15

    def next(self) -> int:
        value = self.state
        value ^= (value << 13) & MASK
        value ^= value >> 7
        value ^= (value << 17) & MASK
        self.state = value & MASK
        return self.state


def bootstrap_interval(values: list[float], policy: str) -> tuple[float, float]:
    seed = SEED ^ stable_hash(policy)
    rng = XorShift64(seed)
    if seed == 0:
        rng = XorShift64(0x9E3779B97F4A7C15)
    boot_means = []
    for _ in range(SAMPLES):
        sample = [values[rng.next() % len(values)] for _ in range(len(values))]
        boot_means.append(mean(sample))
    boot_means.sort()
    return (
        boot_means[math.ceil(0.025 * SAMPLES) - 1],
        boot_means[math.ceil(0.975 * SAMPLES) - 1],
    )


def expected_rows() -> dict[str, dict]:
    task_values = {"uniform": [], "voi": [], "adjoint": []}
    for base, voi_delta, adjoint_delta in zip(BASELINE, VOI, ADJOINT, strict=True):
        task_values["uniform"].append(median([base + j for j in JITTER]))
        task_values["voi"].append(median([base + voi_delta + j for j in JITTER]))
        task_values["adjoint"].append(median([base + adjoint_delta + j for j in JITTER]))

    results = {}
    for policy, values in task_values.items():
        deltas = [score - base for score, base in zip(values, task_values["uniform"], strict=True)]
        low, high = bootstrap_interval(deltas, policy)
        results[policy] = {
            "mean": mean(values),
            "median": median(values),
            "p10": nearest_rank(values, 0.10),
            "worst": min(values),
            "mean_delta": mean(deltas),
            "median_delta": median(deltas),
            "p10_delta": nearest_rank(deltas, 0.10),
            "worst_delta": min(deltas),
            "better": sum(delta > 1e-12 for delta in deltas),
            "tied": sum(abs(delta) <= 1e-12 for delta in deltas),
            "worse": sum(delta < -1e-12 for delta in deltas),
            "ci_low": low,
            "ci_high": high,
        }
    return results


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-policy-aggregate-independent.py aggregate.csv", file=sys.stderr)
        return 2
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 3, f"expected 3 policies, got {len(rows)}")
    expected = expected_rows()
    seen = set()
    for row in rows:
        policy = row["policy_id"]
        require(policy in expected, f"unknown policy {policy}")
        require(policy not in seen, f"duplicate policy {policy}")
        seen.add(policy)
        require(row["baseline_policy_id"] == "uniform", "baseline mismatch")
        require(int(row["task_count"]) == 10, "task count mismatch")
        require(int(row["replicates_per_task"]) == 3, "replicate count mismatch")
        for field, key in [
            ("mean_task_median_normalized_hypervolume", "mean"),
            ("median_task_median_normalized_hypervolume", "median"),
            ("p10_task_median_normalized_hypervolume", "p10"),
            ("worst_task_normalized_hypervolume", "worst"),
            ("mean_paired_delta_vs_baseline", "mean_delta"),
            ("median_paired_delta_vs_baseline", "median_delta"),
            ("p10_paired_delta_vs_baseline", "p10_delta"),
            ("worst_task_delta_vs_baseline", "worst_delta"),
            ("bootstrap_95_ci_low", "ci_low"),
            ("bootstrap_95_ci_high", "ci_high"),
        ]:
            actual, target = float(row[field]), expected[policy][key]
            require(math.isclose(actual, target, rel_tol=1e-10, abs_tol=1e-12),
                    f"{policy} {field}: {actual} != {target}")
        for field, key in [
            ("tasks_better", "better"), ("tasks_tied", "tied"), ("tasks_worse", "worse")
        ]:
            require(int(row[field]) == expected[policy][key], f"{policy} {field} mismatch")
    require(seen == set(expected), "policy set incomplete")
    print("independent_policy_aggregate_oracle=pass tasks=10 replicates=3 policies=3 cluster_bootstrap=10000")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"independent_policy_aggregate_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
