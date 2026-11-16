#!/usr/bin/env python3
"""Independent benchmark-only CP-SAT known-answer check for FJSPLib FJSP/FAJSP.

The implementation is deliberately isolated from the Rust implementation.
It parses the documented text formats independently and builds its own CP-SAT
optional-interval model. It is a verification oracle, not a production path.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from ortools.sat.python import cp_model

Operation = list[tuple[int, int]]


def meaningful_lines(path: Path) -> list[str]:
    lines = []
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        content = raw_line.split("#", maxsplit=1)[0].strip()
        if content:
            lines.append(content)
    if not lines:
        raise ValueError("instance input is empty")
    return lines


def parse_fjsp(path: Path) -> tuple[int, list[Operation], list[tuple[int, int]]]:
    lines = meaningful_lines(path)
    header = lines[0].split()
    if len(header) != 3:
        raise ValueError("FJSP header must have jobs, machines and average flexibility")

    job_count, machine_count = int(header[0]), int(header[1])
    if job_count <= 0 or machine_count <= 0 or len(lines) != job_count + 1:
        raise ValueError("FJSP header counts do not match supplied job records")

    operations: list[Operation] = []
    precedences: list[tuple[int, int]] = []
    for line_number, line in enumerate(lines[1:], start=2):
        tokens = [int(token) for token in line.split()]
        cursor = 0
        if not tokens:
            raise ValueError(f"line {line_number}: missing operation count")
        operation_count = tokens[cursor]
        cursor += 1
        if operation_count <= 0:
            raise ValueError(f"line {line_number}: operation count must be positive")

        job_ids = []
        for _ in range(operation_count):
            if cursor >= len(tokens):
                raise ValueError(f"line {line_number}: missing alternative count")
            alternative_count = tokens[cursor]
            cursor += 1
            if alternative_count <= 0 or cursor + 2 * alternative_count > len(tokens):
                raise ValueError(f"line {line_number}: invalid alternative count")

            alternatives: Operation = []
            seen: set[int] = set()
            for _ in range(alternative_count):
                machine = tokens[cursor] - 1
                duration = tokens[cursor + 1]
                cursor += 2
                if machine < 0 or machine >= machine_count or duration <= 0 or machine in seen:
                    raise ValueError(f"line {line_number}: invalid machine/duration alternative")
                seen.add(machine)
                alternatives.append((machine, duration))
            job_ids.append(len(operations))
            operations.append(alternatives)

        if cursor != len(tokens):
            raise ValueError(f"line {line_number}: unexpected trailing tokens")
        precedences.extend(zip(job_ids, job_ids[1:]))

    return machine_count, operations, precedences


def parse_fajsp(path: Path) -> tuple[int, list[Operation], list[tuple[int, int]]]:
    lines = meaningful_lines(path)
    header = lines[0].split()
    if len(header) != 3:
        raise ValueError("FAJSP header must have operations, precedence edges and machines")

    operation_count, precedence_count, machine_count = map(int, header)
    if operation_count <= 0 or machine_count <= 0:
        raise ValueError("FAJSP operation and machine counts must be positive")
    if len(lines) != 1 + precedence_count + operation_count:
        raise ValueError("FAJSP record count does not match its header")

    precedences: list[tuple[int, int]] = []
    for line_number, line in enumerate(lines[1:1 + precedence_count], start=2):
        fields = [int(token) for token in line.split()]
        if len(fields) != 3:
            raise ValueError(f"line {line_number}: precedence record must have three fields")
        before, after, _job_id = fields
        if before < 0 or after < 0 or before >= operation_count or after >= operation_count:
            raise ValueError(f"line {line_number}: precedence endpoint out of range")
        precedences.append((before, after))

    operations: list[Operation] = []
    for line_number, line in enumerate(lines[1 + precedence_count:], start=2 + precedence_count):
        tokens = [int(token) for token in line.split()]
        if not tokens:
            raise ValueError(f"line {line_number}: missing alternative count")
        alternative_count = tokens[0]
        if alternative_count <= 0 or len(tokens) != 1 + 2 * alternative_count:
            raise ValueError(f"line {line_number}: malformed alternative record")
        alternatives: Operation = []
        seen: set[int] = set()
        cursor = 1
        for _ in range(alternative_count):
            machine = tokens[cursor]
            duration = tokens[cursor + 1]
            cursor += 2
            if machine < 0 or machine >= machine_count or duration <= 0 or machine in seen:
                raise ValueError(f"line {line_number}: invalid machine/duration alternative")
            seen.add(machine)
            alternatives.append((machine, duration))
        operations.append(alternatives)

    return machine_count, operations, precedences


def solve_instance(
    path: Path, instance_format: str, time_limit_seconds: float
) -> dict[str, object]:
    if instance_format == "fjsp":
        machine_count, operations, precedences = parse_fjsp(path)
    else:
        machine_count, operations, precedences = parse_fajsp(path)

    horizon = sum(max(duration for _, duration in operation) for operation in operations)
    model = cp_model.CpModel()
    starts = [model.NewIntVar(0, horizon, f"start_{i}") for i in range(len(operations))]
    ends = [model.NewIntVar(0, horizon, f"end_{i}") for i in range(len(operations))]
    machine_intervals: list[list[cp_model.IntervalVar]] = [[] for _ in range(machine_count)]
    choices: list[list[tuple[int, cp_model.IntVar]]] = []

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

    for before, after in precedences:
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
        "format": instance_format,
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
    parser.add_argument("--format", choices=("fjsp", "fajsp"), default="fjsp")
    parser.add_argument("--time-limit-seconds", type=float, default=30.0)
    args = parser.parse_args()
    if args.time_limit_seconds <= 0:
        parser.error("--time-limit-seconds must be positive")
    print(json.dumps(
        solve_instance(args.instance, args.format, args.time_limit_seconds),
        sort_keys=True,
    ))


if __name__ == "__main__":
    main()
