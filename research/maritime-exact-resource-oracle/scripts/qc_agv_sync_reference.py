#!/usr/bin/env python3
"""Independent exact reference for event-synchronized QC–AGV routes."""
from __future__ import annotations

import argparse
import csv
import itertools
from pathlib import Path


def parse_fixture(path: Path):
    instance = None
    tasks: dict[str, dict] = {}
    routes: dict[str, list[tuple[str, int]]] = {}

    for line_number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        kind = fields[0]
        if kind == "INSTANCE" and len(fields) == 5:
            if instance is not None:
                raise AssertionError("duplicate instance header")
            instance = {
                "id": fields[1],
                "gamma": int(fields[2]),
                "qc_weight": int(fields[3]),
                "agv_weight": int(fields[4]),
            }
        elif kind == "TASK" and len(fields) == 7:
            task_id = fields[1]
            if task_id in tasks:
                raise AssertionError(f"duplicate task {task_id}")
            tasks[task_id] = {
                "qc": fields[2],
                "qc_release": int(fields[3]),
                "nominal": int(fields[4]),
                "deviation": int(fields[5]),
                "qc_predecessor": None if fields[6] == "-" else fields[6],
            }
        elif kind == "ROUTE" and len(fields) == 2:
            if fields[1] in routes:
                raise AssertionError(f"duplicate route {fields[1]}")
            routes[fields[1]] = []
        elif kind == "STEP" and len(fields) == 4:
            if fields[1] not in routes:
                raise AssertionError(f"STEP before ROUTE on line {line_number}")
            routes[fields[1]].append((fields[2], int(fields[3])))
        else:
            raise AssertionError(f"malformed fixture row {line_number}: {fields}")

    if instance is None or not tasks or not routes:
        raise AssertionError("incomplete synchronization fixture")
    if instance["gamma"] > len(tasks):
        raise AssertionError("Gamma exceeds number of tasks")

    for task_id, task in tasks.items():
        if task["nominal"] <= task["deviation"]:
            raise AssertionError(f"invalid service band for {task_id}")
        if task["nominal"] + task["deviation"] > (1 << 32) - 1:
            raise AssertionError(f"service band overflow for {task_id}")
        predecessor = task["qc_predecessor"]
        if predecessor is not None:
            if predecessor not in tasks:
                raise AssertionError(f"unknown QC predecessor {predecessor}")
            if tasks[predecessor]["qc"] != task["qc"]:
                raise AssertionError(f"QC predecessor uses a different QC for {task_id}")

    for route_id, steps in routes.items():
        if {task_id for task_id, _ in steps} != set(tasks) or len(steps) != len(tasks):
            raise AssertionError(f"{route_id}: route must contain every task exactly once")
    return instance, tasks, routes


def uncertainty_vectors(task_ids: list[str], gamma: int):
    vectors = [
        vector for vector in itertools.product((-1, 0, 1), repeat=len(task_ids))
        if sum(abs(value) for value in vector) <= gamma
    ]
    return sorted(vectors)


def simulate(instance, tasks, steps, scenario):
    task_ids = sorted(tasks)
    task_index = {task_id: index for index, task_id in enumerate(task_ids)}
    timings = {}
    total_qc_wait = 0
    total_agv_wait = 0
    makespan = 0

    for index, (task_id, travel_before) in enumerate(steps):
        task = tasks[task_id]
        predecessor = task["qc_predecessor"]
        if predecessor is None:
            qc_ready = task["qc_release"]
        else:
            if predecessor not in timings:
                raise AssertionError(f"QC predecessor {predecessor} unavailable before {task_id}")
            qc_ready = timings[predecessor]["finish"]

        if index == 0:
            agv_ready = travel_before
        else:
            previous_id = steps[index - 1][0]
            agv_ready = timings[previous_id]["finish"] + travel_before

        start = max(qc_ready, agv_ready)
        qc_wait = start - qc_ready
        agv_wait = start - agv_ready
        deviation_sign = scenario[task_index[task_id]]
        service = task["nominal"] + deviation_sign * task["deviation"]
        if service <= 0:
            raise AssertionError(f"non-positive service time for {task_id}")
        finish = start + service
        total_qc_wait += qc_wait
        total_agv_wait += agv_wait
        makespan = max(makespan, finish)
        timings[task_id] = {
            "qc_ready": qc_ready,
            "agv_ready": agv_ready,
            "start": start,
            "finish": finish,
            "qc_wait": qc_wait,
            "agv_wait": agv_wait,
        }

    cost = (
        total_qc_wait * instance["qc_weight"]
        + total_agv_wait * instance["agv_weight"]
    )
    return {
        "scenario": scenario,
        "qc_wait": total_qc_wait,
        "agv_wait": total_agv_wait,
        "cost": cost,
        "makespan": makespan,
        "timings": timings,
    }


def evaluate(instance, tasks, routes):
    task_ids = sorted(tasks)
    scenarios = uncertainty_vectors(task_ids, instance["gamma"])
    route_results = {}
    for route_id, steps in sorted(routes.items()):
        nominal = simulate(instance, tasks, steps, tuple(0 for _ in task_ids))
        evaluations = [
            simulate(instance, tasks, steps, scenario) for scenario in scenarios
        ]
        worst = sorted(evaluations, key=lambda item: (-item["cost"], item["scenario"]))[0]
        route_results[route_id] = {"nominal": nominal, "worst": worst}

    nominal_best = min(
        route_results,
        key=lambda route_id: (route_results[route_id]["nominal"]["cost"], route_id),
    )
    robust_best = min(
        route_results,
        key=lambda route_id: (route_results[route_id]["worst"]["cost"], route_id),
    )
    return scenarios, route_results, nominal_best, robust_best


def receipt_lines(instance, scenarios, route_results, nominal_best, robust_best):
    lines = [
        "QC_AGV_SYNC_RECEIPT\t0.1",
        f"INSTANCE\t{instance['id']}\t{instance['gamma']}\t{len(scenarios)}",
    ]
    for route_id, values in sorted(route_results.items()):
        nominal = values["nominal"]
        worst = values["worst"]
        witness = ",".join(map(str, worst["scenario"]))
        lines.append(
            "ROUTE\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}".format(
                route_id,
                nominal["cost"],
                worst["cost"],
                worst["qc_wait"],
                worst["agv_wait"],
                worst["makespan"],
                witness,
                nominal["makespan"],
            )
        )
    lines.append(f"NOMINAL_BEST\t{nominal_best}")
    lines.append(f"ROBUST_BEST\t{robust_best}")
    lines.append("SUMMARY\tPASS")
    return lines


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument(
        "--fixture",
        type=Path,
        default=Path("research/maritime-exact-resource-oracle/fixtures/qc-agv-sync.tsv"),
    )
    args = parser.parse_args()

    instance, tasks, routes = parse_fixture(args.fixture)
    scenarios, route_results, nominal_best, robust_best = evaluate(instance, tasks, routes)
    if nominal_best != "BA" or robust_best != "AB":
        raise AssertionError(
            f"frozen trade-off changed: nominal={nominal_best}, robust={robust_best}"
        )
    if len(scenarios) != 5:
        raise AssertionError(f"expected 5 Gamma=1 scenarios, got {len(scenarios)}")
    expected = receipt_lines(instance, scenarios, route_results, nominal_best, robust_best)
    actual = args.receipt.read_text(encoding="utf-8").splitlines()
    if actual != expected:
        raise AssertionError(f"QC–AGV sync Rust/reference mismatch\nexpected={expected}\nactual={actual}")

    print("QC_AGV_SYNC_REFERENCE\tPASS\t5/5")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
