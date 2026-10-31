#!/usr/bin/env python3
"""Independent replay oracle for the executed ZDT1/ZDT2 optimizer qualification lane."""
from __future__ import annotations
import csv
import math
import statistics
import sys
from collections import defaultdict
from pathlib import Path

REFERENCE = (1.1, 1.1)
CHECKPOINTS = (100, 250, 500, 1000)
SEEDS = (101, 202, 303, 404, 505)
TASKS = ("zdt1", "zdt2", "zdt3", "zdt4")
POLICIES = ("uniform_random", "nsga2", "gde3")
DIMENSIONS = 30
ALGORITHM_CONFIG_IDS = {
    "uniform_random": "uniform-random-sampling-v0.1",
    "nsga2": "nsga2-sbx15-pm20-pc0.5-v0.1",
    "gde3": "gde3-de-rand1-f0.5-cr0.9-v0.1",
}
INITIAL_POPULATION_BASIS = "common_seeded_uniform_v0.1"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def objective(task: str, x: list[float]) -> tuple[float, float, float]:
    require(len(x) == DIMENSIONS, f"expected {DIMENSIONS} variables, got {len(x)}")
    require(
        all(
            math.isfinite(value)
            and (
                (-5.0 <= value <= 5.0 if task == "zdt4" and index > 0
                 else 0.0 <= value <= 1.0)
            )
            for index, value in enumerate(x)
        ),
        "decision variable outside task-specific domain",
    )
    f1 = x[0]
    if task == "zdt4":
        g = 1.0 + 10.0 * (len(x) - 1) + sum(
            value**2 - 10.0 * math.cos(4.0 * math.pi * value)
            for value in x[1:]
        )
    else:
        g = 1.0 + 9.0 * sum(x[1:]) / (len(x) - 1)
    ratio = f1 / g
    if task == "zdt1":
        f2 = g * (1.0 - math.sqrt(ratio))
    elif task == "zdt2":
        f2 = g * (1.0 - ratio * ratio)
    elif task == "zdt3":
        f2 = g * (
            1.0 - math.sqrt(ratio)
            - ratio * math.sin(10.0 * math.pi * f1)
        )
    elif task == "zdt4":
        f2 = g * (1.0 - math.sqrt(ratio))
    else:
        raise ValueError(f"unknown task {task}")
    return f1, f2, g


def archive_points(points: list[tuple[float, float]]) -> list[tuple[float, float]]:
    result: list[tuple[float, float]] = []
    best = math.inf
    for point in sorted(points, key=lambda p: (p[0], p[1])):
        if point[1] < best:
            best = point[1]
            result.append(point)
    return result


def hypervolume(points: list[tuple[float, float]]) -> float:
    eligible = [
        p for p in points
        if 0.0 <= p[0] < REFERENCE[0] and 0.0 <= p[1] < REFERENCE[1]
    ]
    archive = archive_points(eligible)
    total = 0.0
    for index, point in enumerate(archive):
        next_x = archive[index + 1][0] if index + 1 < len(archive) else REFERENCE[0]
        total += (next_x - point[0]) * (REFERENCE[1] - point[1])
    return max(0.0, total)


ZDT3_INTERVALS = (
    (0.0, 0.0830015349),
    (0.1822287280, 0.2577623634),
    (0.4093136748, 0.4538821041),
    (0.6183967944, 0.6525117038),
    (0.8233317983, 0.8518328654),
)


def zdt3_primitive(x: float) -> float:
    frequency = 10.0 * math.pi
    return (
        (REFERENCE[1] - 1.0) * x
        + (2.0 / 3.0) * x**1.5
        - x * math.cos(frequency * x) / frequency
        + math.sin(frequency * x) / frequency**2
    )


def zdt3_y(x: float) -> float:
    return 1.0 - math.sqrt(x) - x * math.sin(10.0 * math.pi * x)


def exact_hv(task: str) -> float:
    if task != "zdt3":
        curve_integral = (
            (REFERENCE[1] - 1.0)
            + (2.0 / 3.0 if task in ("zdt1", "zdt4") else 1.0 / 3.0)
        )
        return curve_integral + (REFERENCE[0] - 1.0) * REFERENCE[1]

    area = 0.0
    for index, (start, end) in enumerate(ZDT3_INTERVALS):
        if index:
            previous_end = ZDT3_INTERVALS[index - 1][1]
            area += (start - previous_end) * (REFERENCE[1] - zdt3_y(previous_end))
        area += zdt3_primitive(end) - zdt3_primitive(start)
    last_end = ZDT3_INTERVALS[-1][1]
    return area + (REFERENCE[0] - last_end) * (REFERENCE[1] - zdt3_y(last_end))


def pareto_y(task: str, x: float) -> float:
    if task in ("zdt1", "zdt4"):
        return 1.0 - math.sqrt(x)
    if task == "zdt2":
        return 1.0 - x * x
    return zdt3_y(x)


def reference_points(task: str) -> list[tuple[float, float]]:
    if task == "zdt3":
        counts = (41, 40, 40, 40, 40)
        points = []
        for (start, end), count in zip(ZDT3_INTERVALS, counts):
            for index in range(count):
                x = start + (end - start) * index / (count - 1)
                points.append((x, zdt3_y(x)))
        return points
    return [(index / 200.0, pareto_y(task, index / 200.0)) for index in range(201)]


def igd(task: str, archive: list[tuple[float, float]]) -> float:
    require(bool(archive), "empty archive has infinite IGD")
    reference = reference_points(task)
    total = sum(
        min(math.hypot(px - x, py - y) for px, py in archive)
        for x, y in reference
    )
    return total / len(reference)


def load_candidates(path: Path) -> dict[tuple[str, str, int], list[dict]]:
    grouped: dict[tuple[str, str, int], list[dict]] = defaultdict(list)
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            key = (row["task_id"], row["policy_id"], int(row["seed"]))
            vector = [
                float(value)
                for value in row["decision_vector_semicolon_separated"].split(";")
            ]
            f1, f2, g = objective(key[0], vector)
            require(
                math.isclose(float(row["objective_1"]), f1, rel_tol=1e-12, abs_tol=1e-14),
                f"objective_1 mismatch {key} evaluation {row['evaluation']}",
            )
            require(
                math.isclose(float(row["objective_2"]), f2, rel_tol=1e-12, abs_tol=1e-14),
                f"objective_2 mismatch {key} evaluation {row['evaluation']}",
            )
            require(
                math.isclose(float(row["g_value"]), g, rel_tol=1e-12, abs_tol=1e-14),
                f"g mismatch {key} evaluation {row['evaluation']}",
            )
            row["_vector"] = vector
            grouped[key].append(row)
    return grouped


def verify_summaries(path: Path, candidates: dict[tuple[str, str, int], list[dict]]) -> dict:
    aggregate: dict[tuple[str, str], list[float]] = defaultdict(list)
    seen: set[tuple[tuple[str, str, int], int]] = set()
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            key = (row["task_id"], row["policy_id"], int(row["seed"]))
            checkpoint = int(row["evaluation_budget"])
            require((key, checkpoint) not in seen, "duplicate summary checkpoint")
            seen.add((key, checkpoint))
            trace = candidates.get(key)
            require(trace is not None, f"missing candidate trace group {key}")
            require(len(trace) == 1000, f"{key} expected 1000 candidate evaluations")
            trace.sort(key=lambda item: int(item["evaluation"]))
            require(
                [int(item["evaluation"]) for item in trace] == list(range(1, 1001)),
                f"{key} has missing or duplicate evaluation identifiers",
            )
            require(checkpoint in CHECKPOINTS, f"unexpected checkpoint {checkpoint}")
            require(row["algorithm_config_id"] == ALGORITHM_CONFIG_IDS[key[1]],
                    f"algorithm config ID mismatch for {key}")
            require(row["initial_population_basis"] == INITIAL_POPULATION_BASIS,
                    f"common-initialization metadata missing for {key}")
            prefix = trace[:checkpoint]
            points = [(float(item["objective_1"]), float(item["objective_2"])) for item in prefix]
            archive = archive_points(points)
            hv = hypervolume(archive)
            truth = exact_hv(key[0])
            fraction = hv / truth
            distance = igd(key[0], archive)
            mean_g = statistics.fmean(float(item["g_value"]) for item in prefix)
            targets = {
                "decision_dimensions": float(DIMENSIONS),
                "population_size": 40.0,
                "maximum_evaluations": 1000.0,
                "actual_evaluations": float(checkpoint),
                "archive_size": float(len(archive)),
                "hypervolume": hv,
                "exact_front_hypervolume": truth,
                "hypervolume_fraction_of_exact_front": fraction,
                "inverted_generational_distance": distance,
                "mean_g_value": mean_g,
            }
            for field, expected in targets.items():
                actual = float(row[field])
                require(
                    math.isclose(actual, expected, rel_tol=2e-10, abs_tol=2e-12),
                    f"{key} checkpoint={checkpoint} {field}: {actual} != {expected}",
                )
            require(
                row["evidence_level"]
                == "synthetic_analytic_optimizer_qualification_not_aerodynamic_evidence",
                "evidence ceiling missing",
            )
            if checkpoint == CHECKPOINTS[-1]:
                aggregate[(key[0], key[1])].append(fraction)

    expected_groups = {
        (task, policy, seed)
        for task in TASKS
        for policy in POLICIES
        for seed in SEEDS
    }
    require(set(candidates) == expected_groups, "candidate task/policy/seed matrix mismatch")
    require(len(seen) == len(expected_groups) * len(CHECKPOINTS), "summary row count mismatch")
    for key in expected_groups:
        actual_checkpoints = tuple(sorted(
            checkpoint for summary_key, checkpoint in seen if summary_key == key
        ))
        require(actual_checkpoints == CHECKPOINTS, f"incomplete checkpoint curve {key}")
    return {key: statistics.median(values) for key, values in aggregate.items()}


def verify_common_initial_designs(
    candidates: dict[tuple[str, str, int], list[dict]]
) -> None:
    for task in TASKS:
        for seed in SEEDS:
            reference = None
            for policy in POLICIES:
                rows = sorted(
                    candidates[(task, policy, seed)],
                    key=lambda item: int(item["evaluation"]),
                )
                initial = tuple(tuple(row["_vector"]) for row in rows[:40])
                if reference is None:
                    reference = initial
                else:
                    require(
                        initial == reference,
                        f"common seeded initial population mismatch task={task} seed={seed} policy={policy}",
                    )


def main() -> int:
    if len(sys.argv) != 3:
        print(
            "usage: verify-optimizer-qualification-independent.py summary.csv candidate-trace.csv",
            file=sys.stderr,
        )
        return 2
    candidates = load_candidates(Path(sys.argv[2]))
    require(
        sum(len(group) for group in candidates.values()) == 60_000,
        "expected 30,000 evaluated decision vectors",
    )
    verify_common_initial_designs(candidates)
    aggregate = verify_summaries(Path(sys.argv[1]), candidates)
    print(
        "independent_optimizer_qualification_oracle=pass "
        "tasks=4 policies=3 seeds=5 checkpoints=4 evaluations=60000"
    )
    for task in TASKS:
        random_hv = aggregate[(task, "uniform_random")]
        nsga_hv = aggregate[(task, "nsga2")]
        print(
            f"recomputed_median_final_hv_fraction task={task} "
            f"random={random_hv:.8f} nsga2={nsga_hv:.8f}"
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"independent_optimizer_qualification_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
