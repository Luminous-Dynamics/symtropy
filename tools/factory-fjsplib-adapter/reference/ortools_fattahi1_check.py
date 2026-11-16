#!/usr/bin/env python3
"""Independent benchmark-only CP-SAT known-answer check for FJSPLib FJSP.

This script is deliberately isolated from the Rust implementation. It parses
only the documented text FJSP input used by the fixture and builds a small
flexible job-shop makespan model in Google OR-Tools CP-SAT. It is a verification
oracle, not part of Symtropy's production/runtime path.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from ortools.sat.python import cp_model


def parse_fjsp(path: Path) -> tuple[int, list[list[list[tuple[int, int]]]]]:
    lines = []
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        content = raw_line.split("#", maxsplit=1)[0].strip()
        if content:
            lines.append(content)

    if not lines:
        raise ValueError("FJSP input is empty")

    header = lines[0].split()
    if len(header) != 3:
        raise ValueError("FJSP header must have jobs, machines and average flexibility")

    job_count, machine_count = int(header[0]), int(header[1])
    if job_count <= 0 or machine_count <= 0 or len(lines) != job_count + 1:
        raise ValueError("FJSP header counts do not match supplied job records")

    jobs: list[list[list[tuple[int, int]]]] = []
    for line_number, line in enumerate(lines[1:], start=2):
        tokens = [int(token) for token in line.split()]
        cursor = 0
        if not tokens:
            raise ValueError(f"line {line_number}: missing operation count")
        operation_count = tokens[cursor]
        cursor += 1
        if operation_count <= 0:
            raise ValueError(f"line {line_number}: operation count must be positive")

        operations: list[list[tuple[int, int]]] = []
        for _ in range(operation_count):
            if cursor >= len(tokens):
                raise ValueError(f"line {line_number}: missing alternative count")
            alternative_count = tokens[cursor]
            cursor += 1
            if alternative_count <= 0 or cursor + 2 * alternative_count > len(tokens):
                raise ValueError(f"line {line_number}: invalid alternative count")

            alternatives: list[tuple[int, int]] = []
            seen: set[int] = set()
            for _ in range(alternative_count):
                machine_one_based = tokens[cursor]
                duration = tokens[cursor + 1]
                cursor += 2
                machine = machine_one_based - 1
                if machine < 0 or machine >= machine_count or duration <= 0 or machine in seen:
                    raise ValueError(f"line {line_number}: invalid machine/duration alternative")
                seen.add(machine)
                alternatives.append((machine, duration))
            operations.append(alternatives)

        if cursor != len(tokens):
            raise ValueError(f"line {line_number}: unexpected trailing tokens")
        jobs.append(operations)

    return machine_count, jobs


def solve_fjsp(path: Path, time_limit_seconds: float) -> dict[str, object]:
    machine_count, jobs = parse_fjsp(path)
    operations = [operation for job in jobs for operation in job]
    durations = [max(duration for _, duration in operation) for operation in operations]
    horizon = sum(durations)

    model = cp_model.CpModel()
    starts = [model.NewIntVar(0, horizon, f"start_{i}") for i in range(len(operations))]
    ends = [model.NewIntVar(0, horizon, f"end_{i}") for i in range(len(operations))]
    machine_intervals: list[list[cp_model.IntervalVar]] = [[] for _ in range(machine_count)]
    choices: list[list[tuple[int, cp_model.IntVar]]] = []

    operation_id = 0
    job_precedences: list[tuple[int, int]] = []
    for job in jobs:
        job_ids = list(range(operation_id, operation_id + len(job)))
        job_precedences.extend(zip(job_ids, job_ids[1:]))
        operation_id += len(job)

    for operation_index, alternatives in enumerate(operations):
        operation_choices: list[tuple[int, cp_model.IntVar]] = []
        presence_vars = []
        for machine_id, duration in alternatives:
            present = model.NewBoolVar(f"op_{operation_index}_on_machine_{machine_id}")
            interval = model.NewOptionalIntervalVar(
                starts[operation_index],
                duration,
                ends[operation_index],
                present,
                f"interval_{operation_index}_{machine_id}",
            )
            machine_intervals[machine_id].append(interval)
            operation_choices.append((machine_id, present))
            presence_vars.append(present)
        model.AddExactlyOne(presence_vars)
        choices.append(operation_choices)

    for intervals in machine_intervals:
        if intervals:
            model.AddNoOverlap(intervals)

    for before, after in job_precedences:
        model.Add(starts[after] >= ends[before])

    makespan = model.NewIntVar(0, horizon, "makespan")
    model.AddMaxEquality(makespan, ends)
    model.Minimize(makespan)

    solver = cp_model.CpSolver()
    solver.parameters.max_time_in_seconds = time_limit_seconds
    solver.parameters.num_search_workers = 1
    solver.parameters.random_seed = 0
    status = solver.Solve(model)

    if status != cp_model.OPTIMAL:
        raise RuntimeError(
            f"CP-SAT did not prove optimality: {solver.StatusName(status)}; "
            f"best_bound={solver.BestObjectiveBound()}"
        )

    selected_machines = []
    for operation_choices in choices:
        selected = [machine for machine, present in operation_choices if solver.Value(present)]
        if len(selected) != 1:
            raise RuntimeError("CP-SAT result does not select exactly one machine per operation")
        selected_machines.append(selected[0])

    return {
        "status": solver.StatusName(status),
        "objective": int(round(solver.ObjectiveValue())),
        "best_bound": int(round(solver.BestObjectiveBound())),
        "operation_count": len(operations),
        "machine_count": machine_count,
        "selected_machines_zero_based": selected_machines,
        "solver": "OR-Tools CP-SAT 9.15.6755",
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("instance", type=Path)
    parser.add_argument("--time-limit-seconds", type=float, default=30.0)
    args = parser.parse_args()
    if args.time_limit_seconds <= 0:
        parser.error("--time-limit-seconds must be positive")
    result = solve_fjsp(args.instance, args.time_limit_seconds)
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
