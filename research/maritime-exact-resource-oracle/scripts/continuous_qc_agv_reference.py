#!/usr/bin/env python3
"""Independent active-region LP reference for continuous QC–AGV timing."""
from __future__ import annotations

import argparse
import csv
import heapq
import itertools
from pathlib import Path


TOLERANCE = 1e-6


def fmt(value: float) -> str:
    if abs(value) < 0.0000005:
        value = 0.0
    return f"{value:.6f}"


def add(left, right):
    return (left[0] + right[0], [a + b for a, b in zip(left[1], right[1])])


def sub(left, right):
    return (left[0] - right[0], [a - b for a, b in zip(left[1], right[1])])


def scale(value, factor):
    return (value[0] * factor, [x * factor for x in value[1]])


def solve_square(matrix, rhs, eps=1e-8):
    n = len(rhs)
    aug = [list(row) + [rhs[i]] for i, row in enumerate(matrix)]
    for col in range(n):
        pivot = max(range(col, n), key=lambda row: abs(aug[row][col]))
        if abs(aug[pivot][col]) <= eps:
            return None
        aug[col], aug[pivot] = aug[pivot], aug[col]
        divisor = aug[col][col]
        for j in range(col, n + 1):
            aug[col][j] /= divisor
        for row in range(n):
            if row == col:
                continue
            factor = aug[row][col]
            if abs(factor) <= eps:
                continue
            for j in range(col, n + 1):
                aug[row][j] -= factor * aug[col][j]
    return [aug[i][n] for i in range(n)]


def parse_fixture(path: Path):
    instance_id = None
    gamma = qc_weight = agv_weight = None
    tasks = []
    plan_ids = set()
    routes = {}

    with path.open("r", encoding="utf-8", newline="") as handle:
        for line_number, raw in enumerate(handle, 1):
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            fields = line.split("\t")
            kind = fields[0]
            if kind == "INSTANCE" and len(fields) == 5:
                if instance_id is not None:
                    raise AssertionError("duplicate INSTANCE")
                instance_id = fields[1]
                gamma, qc_weight, agv_weight = map(float, fields[2:5])
            elif kind == "TASK" and len(fields) == 9:
                direction = fields[7]
                if direction not in {"LOADING", "UNLOADING"}:
                    raise AssertionError(f"invalid direction at line {line_number}")
                tasks.append({
                    "id": fields[1],
                    "qc": fields[2],
                    "qc_pred": None if fields[3] == "-" else fields[3],
                    "qc_release": float(fields[4]),
                    "nominal": float(fields[5]),
                    "deviation": float(fields[6]),
                    "direction": direction,
                    "loaded_after": float(fields[8]),
                })
            elif kind == "PLAN" and len(fields) == 2:
                if fields[1] in plan_ids:
                    raise AssertionError(f"duplicate plan {fields[1]}")
                plan_ids.add(fields[1])
                routes[fields[1]] = {}
            elif kind == "ROUTE" and len(fields) == 5:
                plan_id, vehicle_id, task_id = fields[1:4]
                if plan_id not in plan_ids:
                    raise AssertionError(f"unknown plan {plan_id}")
                routes[plan_id].setdefault(vehicle_id, []).append(
                    (task_id, float(fields[4]))
                )
            else:
                raise AssertionError(f"malformed fixture line {line_number}")

    if instance_id is None:
        raise AssertionError("missing INSTANCE")
    return instance_id, gamma, qc_weight, agv_weight, tasks, routes


def validate_structure(tasks, plan_routes):
    n = len(tasks)
    index = {task["id"]: i for i, task in enumerate(tasks)}
    if len(index) != n:
        raise AssertionError("duplicate task ID")

    vehicle_pred = {}
    travel_before = {}
    route_task_ids = set()
    for vehicle_id in sorted(plan_routes):
        steps = plan_routes[vehicle_id]
        if not steps:
            raise AssertionError(f"empty route {vehicle_id}")
        previous = None
        for task_id, travel in steps:
            if task_id not in index:
                raise AssertionError(f"unknown route task {task_id}")
            if task_id in route_task_ids:
                raise AssertionError(f"duplicate route task {task_id}")
            if travel < 0:
                raise AssertionError("negative travel")
            route_task_ids.add(task_id)
            vehicle_pred[task_id] = previous
            travel_before[task_id] = travel
            previous = task_id
    if route_task_ids != set(index):
        raise AssertionError("route tasks do not cover the task set")

    qc_pred = {}
    for task in tasks:
        if task["nominal"] - task["deviation"] < 1.0:
            raise AssertionError(f"invalid service band for {task['id']}")
        if task["direction"] == "LOADING" and abs(task["loaded_after"]) > 1e-8:
            raise AssertionError(f"loading task has post-service delivery: {task['id']}")
        if task["loaded_after"] < 0:
            raise AssertionError(f"negative delivery leg: {task['id']}")
        predecessor = task["qc_pred"]
        if predecessor is not None:
            if predecessor not in index:
                raise AssertionError(f"missing QC predecessor: {predecessor}")
            if tasks[index[predecessor]]["qc"] != task["qc"]:
                raise AssertionError("QC predecessor belongs to another crane")
        qc_pred[task["id"]] = predecessor

    # Fixed QC precedence must form exactly one linear chain per crane.
    qc_groups = {}
    qc_successors = {}
    qc_roots = {}
    for task in tasks:
        qc_groups.setdefault(task["qc"], []).append(task["id"])
        predecessor = task["qc_pred"]
        if predecessor is None:
            qc_roots.setdefault(task["qc"], []).append(task["id"])
        else:
            qc_successors.setdefault(predecessor, []).append(task["id"])
    for qc_id, group_tasks in qc_groups.items():
        roots = qc_roots.get(qc_id, [])
        if len(roots) != 1 or any(
            len(qc_successors.get(task_id, [])) > 1 for task_id in group_tasks
        ):
            raise AssertionError(f"QC tasks do not form one linear order: {qc_id}")
        visited = set()
        current = roots[0]
        while True:
            if current in visited:
                raise AssertionError(f"QC precedence cycle: {qc_id}")
            visited.add(current)
            next_tasks = qc_successors.get(current, [])
            if not next_tasks:
                break
            if len(next_tasks) != 1:
                raise AssertionError(f"branching QC precedence: {qc_id}")
            current = next_tasks[0]
        if len(visited) != len(group_tasks):
            raise AssertionError(f"disconnected QC precedence: {qc_id}")

    indegree = {task["id"]: 0 for task in tasks}
    successors = {task["id"]: set() for task in tasks}
    for task in tasks:
        preds = {
            value for value in (qc_pred[task["id"]], vehicle_pred[task["id"]])
            if value is not None
        }
        for predecessor in preds:
            indegree[task["id"]] += 1
            successors[predecessor].add(task["id"])

    ready = [task_id for task_id, degree in indegree.items() if degree == 0]
    heapq.heapify(ready)
    topo = []
    while ready:
        task_id = heapq.heappop(ready)
        topo.append(task_id)
        for next_id in sorted(successors[task_id]):
            indegree[next_id] -= 1
            if indegree[next_id] == 0:
                heapq.heappush(ready, next_id)
    if len(topo) != n:
        raise AssertionError("combined QC/vehicle precedence cycle")

    return index, topo, qc_pred, vehicle_pred, travel_before


def build_affine_model(tasks, structure, qc_weight, agv_weight, branch_mask):
    index, topo, qc_pred, vehicle_pred, travel_before = structure
    n = len(tasks)
    qc_finish = {}
    vehicle_release = {}
    objective = (0.0, [0.0] * n)
    branches = [None] * n

    for task_id in topo:
        i = index[task_id]
        task = tasks[i]
        if qc_pred[task_id] is None:
            qc_ready = (task["qc_release"], [0.0] * n)
        else:
            qc_ready = qc_finish[qc_pred[task_id]]
        if vehicle_pred[task_id] is None:
            agv_ready = (travel_before[task_id], [0.0] * n)
        else:
            agv_ready = (
                vehicle_release[vehicle_pred[task_id]][0] + travel_before[task_id],
                vehicle_release[vehicle_pred[task_id]][1][:],
            )

        difference = sub(qc_ready, agv_ready)
        choose_agv = bool(branch_mask & (1 << i))
        if choose_agv:
            branches[i] = (difference[1][:], -difference[0])
        else:
            branches[i] = ([-v for v in difference[1]], difference[0])

        service_coeffs = [0.0] * n
        service_coeffs[i] = task["deviation"]
        service = (task["nominal"], service_coeffs)
        start = agv_ready if choose_agv else qc_ready
        finish = add(start, service)
        if task["direction"] == "LOADING":
            release = start
        else:
            release = add(finish, (task["loaded_after"], [0.0] * n))

        objective = add(
            objective,
            add(
                scale(sub(start, qc_ready), qc_weight),
                scale(sub(start, agv_ready), agv_weight),
            ),
        )
        qc_finish[task_id] = finish
        vehicle_release[task_id] = release

    return objective, branches


def replay(tasks, structure, witness, qc_weight, agv_weight):
    index, topo, qc_pred, vehicle_pred, travel_before = structure
    task_map = {task["id"]: task for task in tasks}
    qc_finish = {}
    vehicle_release = {}
    events = []
    qc_wait = 0.0
    agv_wait = 0.0
    makespan = 0.0
    for task_id in topo:
        task = task_map[task_id]
        qc_ready = (
            qc_finish[qc_pred[task_id]]
            if qc_pred[task_id] is not None
            else task["qc_release"]
        )
        agv_ready = (
            vehicle_release[vehicle_pred[task_id]] + travel_before[task_id]
            if vehicle_pred[task_id] is not None
            else travel_before[task_id]
        )
        start = max(qc_ready, agv_ready)
        service = task["nominal"] + task["deviation"] * witness[index[task_id]]
        finish = start + service
        release = start if task["direction"] == "LOADING" else finish + task["loaded_after"]
        task_qc_wait = start - qc_ready
        task_agv_wait = start - agv_ready
        qc_wait += task_qc_wait
        agv_wait += task_agv_wait
        makespan = max(makespan, finish, release)
        qc_finish[task_id] = finish
        vehicle_release[task_id] = release
        events.append((task_id, qc_ready, agv_ready, start, finish, release, task_qc_wait, task_agv_wait))
    return qc_weight * qc_wait + agv_weight * agv_wait, qc_wait, agv_wait, makespan, events


def evaluate_plan(tasks, plan_routes, gamma, qc_weight, agv_weight):
    structure = validate_structure(tasks, plan_routes)
    index, topo, qc_pred, vehicle_pred, travel_before = structure
    n = len(tasks)
    best = None
    combinations_examined = 0
    feasible_active_set_bases = 0

    for sign_mask in range(1 << n):
        signs = [1.0 if not (sign_mask & (1 << i)) else -1.0 for i in range(n)]
        for branch_mask in range(1 << n):
            objective, branches = build_affine_model(
                tasks, structure, qc_weight, agv_weight, branch_mask
            )
            constraints = []
            for i in range(n):
                lower = [0.0] * n
                lower[i] = -1.0
                constraints.append((lower, 0.0))
                upper = [0.0] * n
                upper[i] = 1.0
                constraints.append((upper, 1.0))
            constraints.append(([1.0] * n, gamma))
            for coefficients, rhs in branches:
                constraints.append(
                    ([value * sign for value, sign in zip(coefficients, signs)], rhs)
                )
            objective_coeffs = [
                value * sign for value, sign in zip(objective[1], signs)
            ]

            for active in itertools.combinations(range(len(constraints)), n):
                combinations_examined += 1
                matrix = [constraints[i][0] for i in active]
                rhs = [constraints[i][1] for i in active]
                point = solve_square(matrix, rhs)
                if point is None:
                    continue
                if any(
                    sum(a * x for a, x in zip(coefficients, point)) > bound + 1e-6
                    for coefficients, bound in constraints
                ):
                    continue
                feasible_active_set_bases += 1
                cost = objective[0] + sum(
                    a * x for a, x in zip(objective_coeffs, point)
                )
                witness = [sign * x for sign, x in zip(signs, point)]
                if (
                    best is None
                    or cost > best[0] + 1e-8
                    or (abs(cost - best[0]) <= 1e-8 and tuple(witness) < tuple(best[1]))
                ):
                    best = (cost, witness)

    if best is None:
        raise AssertionError("no feasible active-region vertex")
    worst_cost, witness = best
    replay_result = replay(tasks, structure, witness, qc_weight, agv_weight)
    if abs(replay_result[0] - worst_cost) > 1e-6:
        raise AssertionError(
            f"LP/replay mismatch for {plan_routes}: {worst_cost} vs {replay_result[0]}"
        )
    nominal_witness = [0.0] * n
    nominal = replay(tasks, structure, nominal_witness, qc_weight, agv_weight)
    witness_by_task = {task["id"]: witness[index[task["id"]]] for task in tasks}
    return {
        "nominal_cost": nominal[0],
        "worst_cost": worst_cost,
        "witness": witness_by_task,
        "qc_wait": replay_result[1],
        "agv_wait": replay_result[2],
        "makespan": replay_result[3],
        "events": replay_result[4],
        "combinations": combinations_examined,
        "active_set_bases": feasible_active_set_bases,
    }


def build_receipt(instance_id, gamma, qc_weight, agv_weight, tasks, plans):
    plan_results = {
        plan_id: evaluate_plan(tasks, routes, gamma, qc_weight, agv_weight)
        for plan_id, routes in sorted(plans.items())
    }
    nominal_best = min(
        plan_results,
        key=lambda plan_id: (plan_results[plan_id]["nominal_cost"], plan_id),
    )
    robust_best = min(
        plan_results,
        key=lambda plan_id: (plan_results[plan_id]["worst_cost"], plan_id),
    )
    lines = [
        "CONTINUOUS_QC_AGV_RECEIPT\t0.1",
        f"INSTANCE\t{instance_id}\t{fmt(gamma)}\t{fmt(qc_weight)}\t{fmt(agv_weight)}",
    ]
    for task in tasks:
        lines.append(
            "TASK_INPUT\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}".format(
                task["id"],
                task["qc"],
                task["qc_pred"] or "-",
                fmt(task["qc_release"]),
                fmt(task["nominal"]),
                fmt(task["deviation"]),
                task["direction"],
                fmt(task["loaded_after"]),
            )
        )
    for plan_id, routes in sorted(plans.items()):
        lines.append(f"PLAN_INPUT\t{plan_id}")
        for vehicle_id, steps in sorted(routes.items()):
            for index, (task_id, travel) in enumerate(steps):
                lines.append(
                    f"ROUTE_INPUT\t{plan_id}\t{vehicle_id}\t{index}\t{task_id}\t{fmt(travel)}"
                )
    for plan_id, result in sorted(plan_results.items()):
        witness = ";".join(
            f"{task['id']}={fmt(result['witness'][task['id']])}" for task in tasks
        )
        lines.append(
            f"PLAN_RESULT\t{plan_id}\t{fmt(result['nominal_cost'])}\t{fmt(result['worst_cost'])}"
            f"\t{fmt(result['qc_wait'])}\t{fmt(result['agv_wait'])}\t{fmt(result['makespan'])}\t{witness}"
        )
        lines.append(
            f"PLAN_ENUMERATION\t{plan_id}\t{result['combinations']}\t{result['active_set_bases']}"
        )
        for event in result["events"]:
            task_id, qc_ready, agv_ready, start, finish, release, qc_wait, agv_wait = event
            lines.append(
                f"EVENT\t{plan_id}\t{task_id}\t{fmt(qc_ready)}\t{fmt(agv_ready)}"
                f"\t{fmt(start)}\t{fmt(finish)}\t{fmt(release)}\t{fmt(qc_wait)}\t{fmt(agv_wait)}"
            )
    lines.append(f"WINNERS\t{nominal_best}\t{robust_best}")
    lines.append(f"SUMMARY\tPASS\t{len(plan_results)}/{len(plan_results)}")
    return lines


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument(
        "--fixture",
        type=Path,
        default=Path("research/maritime-exact-resource-oracle/fixtures/continuous-qc-agv.tsv"),
    )
    args = parser.parse_args()
    instance_id, gamma, qc_weight, agv_weight, tasks, plans = parse_fixture(args.fixture)

    # Independently check that a larger nested uncertainty set cannot lower
    # the exact worst-case objective for any fixed plan.
    for plan_id, plan_routes in sorted(plans.items()):
        low = evaluate_plan(tasks, plan_routes, 0.25, qc_weight, agv_weight)
        middle = evaluate_plan(tasks, plan_routes, 0.5, qc_weight, agv_weight)
        high = evaluate_plan(tasks, plan_routes, 1.0, qc_weight, agv_weight)
        if (low["worst_cost"] > middle["worst_cost"] + TOLERANCE or
                middle["worst_cost"] > high["worst_cost"] + TOLERANCE):
            raise AssertionError(f"{plan_id}: worst-case cost is not monotone in Gamma")

    expected = build_receipt(instance_id, gamma, qc_weight, agv_weight, tasks, plans)
    actual = args.receipt.read_text(encoding="utf-8").splitlines()
    if actual != expected:
        raise AssertionError(f"continuous QC–AGV reference mismatch\nexpected={expected}\nactual={actual}")
    print(f"CONTINUOUS_QC_AGV_REFERENCE\tPASS\tplans={len(plans)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
