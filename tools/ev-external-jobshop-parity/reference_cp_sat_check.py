#!/usr/bin/env python3
"""Cross-check the tiny Rust exact oracle against the pinned upstream CP-SAT model."""

import argparse
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import sys

import ortools
from ortools.sat.python import cp_model

UPSTREAM_COMMIT = "d088a26582f5cb79c6b4f2ad94f68ac7c50878cb"
CLAIM_CEILING = (
    "Upstream CP-SAT exact-makespan cross-check on a named pinned tiny static FJSP-SDST case; "
    "not a dispatch-policy, tardiness, factory-performance, or production-release result."
)


class Operation:
    def __init__(self, processing_times):
        self.processing_times = processing_times


class Job:
    def __init__(self, operations):
        self.operations = operations


class TinyJobShop:
    def __init__(self, jobs, setup_times):
        self.jobs = jobs
        self.nr_of_jobs = len(jobs)
        self._sequence_dependent_setup_times = setup_times


def parse_tiny_instance(path):
    lines = [
        line.strip()
        for line in Path(path).read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    if not lines:
        raise ValueError("empty_instance")
    header = lines[0].split()
    if len(header) != 3:
        raise ValueError("reference_adapter_requires_three_header_fields")
    try:
        job_count = int(header[0])
        machine_count = int(header[1])
        declared_average_machine_flexibility = float(header[2])
    except ValueError as error:
        raise ValueError("invalid_job_shop_header") from error
    if (
        job_count != 2
        or machine_count != 2
        or not math.isfinite(declared_average_machine_flexibility)
        or declared_average_machine_flexibility <= 0.0
        or declared_average_machine_flexibility > machine_count
    ):
        raise ValueError("reference_adapter_only_supports_frozen_two_job_two_machine_cases")

    jobs = []
    choices_by_operation = []
    operation_id = 0
    operations_per_job = None
    for job_id in range(job_count):
        values = [int(value) for value in lines[1 + job_id].split()]
        if not values or values[0] != 2:
            raise ValueError("reference_adapter_requires_two_operations_per_job")
        if operations_per_job is None:
            operations_per_job = values[0]
        elif values[0] != operations_per_job:
            raise ValueError("ragged_job_routes_not_supported_by_reference_adapter")
        cursor = 1
        operations = []
        for _position in range(values[0]):
            if cursor >= len(values):
                raise ValueError("missing_operation_alternative_count")
            alternative_count = values[cursor]
            cursor += 1
            if alternative_count <= 0 or alternative_count > machine_count:
                raise ValueError("invalid_machine_alternative_count")
            options = []
            processing_times = {}
            for _ in range(alternative_count):
                if cursor + 1 >= len(values):
                    raise ValueError("truncated_machine_option")
                machine_id = values[cursor] - 1
                processing_time = values[cursor + 1]
                cursor += 2
                if machine_id < 0 or machine_id >= machine_count or processing_time <= 0:
                    raise ValueError("invalid_machine_option")
                if machine_id in processing_times:
                    raise ValueError("duplicate_machine_option")
                options.append((machine_id, processing_time))
                processing_times[machine_id] = processing_time
            choices_by_operation.append(options)
            operations.append(Operation(processing_times))
            operation_id += 1
        if cursor != len(values):
            raise ValueError("unexpected_trailing_job_tokens")
        jobs.append(Job(operations))

    operation_count = operation_id
    observed_average_machine_flexibility = (
        sum(len(options) for options in choices_by_operation) / len(choices_by_operation)
    )
    if abs(observed_average_machine_flexibility - declared_average_machine_flexibility) > 0.0500001:
        raise ValueError("declared_average_machine_flexibility_mismatch")
    setup_rows = [[int(value) for value in line.split()] for line in lines[1 + job_count:]]
    if len(setup_rows) != machine_count * operation_count:
        raise ValueError("setup_row_count_mismatch")
    if any(len(row) != operation_count for row in setup_rows):
        raise ValueError("setup_row_width_mismatch")
    if any(value < 0 for row in setup_rows for value in row):
        raise ValueError("negative_setup_time")
    setup_times = [
        setup_rows[machine_id * operation_count:(machine_id + 1) * operation_count]
        for machine_id in range(machine_count)
    ]
    return TinyJobShop(jobs, setup_times), choices_by_operation


def load_pinned_model(model_path):
    spec = importlib.util.spec_from_file_location("pinned_upstream_fjsp_sdst_cp_sat", model_path)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot_load_pinned_upstream_cp_sat_model")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run_crosscheck(instance_path, model_path, rust_report_path, upstream_commit):
    if upstream_commit != UPSTREAM_COMMIT:
        raise ValueError("unexpected_upstream_revision")
    instance_path = Path(instance_path)
    model_path = Path(model_path)
    rust_report = json.loads(Path(rust_report_path).read_text(encoding="utf-8"))
    oracle = rust_report["exact_oracle"]
    if rust_report["source_commit"] != UPSTREAM_COMMIT:
        raise ValueError("rust_report_source_revision_mismatch")

    env, choices_by_operation = parse_tiny_instance(instance_path)
    upstream_model = load_pinned_model(model_path)
    model, variables = upstream_model.fjsp_sdst_cp_sat_model(env)

    solver = cp_model.CpSolver()
    solver.parameters.max_time_in_seconds = 15.0
    solver.parameters.num_search_workers = 1
    solver.parameters.random_seed = 0
    status = solver.Solve(model)
    status_name = solver.StatusName(status)
    if status != cp_model.OPTIMAL:
        raise RuntimeError(f"reference_solver_not_optimal_{status_name}")

    schedule_by_id = {}
    operations_per_job = max(len(job.operations) for job in env.jobs)
    for job_id, job in enumerate(env.jobs):
        for task_id, _operation in enumerate(job.operations):
            selected = None
            for alternative_id, option in enumerate(choices_by_operation[job_id * operations_per_job + task_id]):
                presence = variables["presences"][(job_id, task_id, alternative_id)]
                if solver.Value(presence):
                    selected = option
                    break
            if selected is None:
                raise RuntimeError(f"no_selected_machine_for_job_{job_id}_task_{task_id}")
            machine_id, processing_time = selected
            operation_id = job_id * operations_per_job + task_id
            start = solver.Value(variables["starts"][(job_id, task_id)])
            schedule_by_id[operation_id] = {
                "operation_id": operation_id,
                "job_id": job_id,
                "position_in_job": task_id,
                "machine_id_upstream": machine_id + 1,
                "setup_time_before_processing": 0,
                "processing_time": processing_time,
                "processing_start": start,
                "completion_time": start + processing_time,
            }

    machine_sequences = []
    for machine_id in range(len(env._sequence_dependent_setup_times)):
        assigned = sorted(
            (
                item for item in schedule_by_id.values()
                if item["machine_id_upstream"] == machine_id + 1
            ),
            key=lambda item: (item["processing_start"], item["operation_id"]),
        )
        sequence = [item["operation_id"] for item in assigned]
        machine_sequences.append(sequence)
        for index, current_id in enumerate(sequence):
            current = schedule_by_id[current_id]
            if index == 0:
                transition_setup = 0
            else:
                previous_id = sequence[index - 1]
                previous = schedule_by_id[previous_id]
                transition_setup = env._sequence_dependent_setup_times[machine_id][previous_id][current_id]
                if current["processing_start"] < previous["completion_time"] + transition_setup:
                    raise RuntimeError("reference_schedule_violates_machine_setup")
            current["setup_time_before_processing"] = transition_setup

    schedule = [schedule_by_id[operation_id] for operation_id in range(len(schedule_by_id))]
    for item in schedule:
        if item["position_in_job"] > 0:
            predecessor = schedule_by_id[item["operation_id"] - 1]
            if item["job_id"] != predecessor["job_id"]:
                raise RuntimeError("unexpected_operation_identifier_layout")
            if item["processing_start"] < predecessor["completion_time"]:
                raise RuntimeError("reference_schedule_violates_job_precedence")

    objective = int(round(solver.ObjectiveValue()))
    if objective != oracle["optimal_makespan"]:
        raise RuntimeError(
            f"reference_makespan_mismatch_cp_sat_{objective}_rust_oracle_{oracle['optimal_makespan']}"
        )
    rust_schedule = oracle["optimal_schedule"]
    if max(item["completion_time"] for item in rust_schedule) != oracle["optimal_makespan"]:
        raise RuntimeError("rust_oracle_schedule_makespan_mismatch")
    for item in rust_schedule:
        if item["completion_time"] != item["processing_start"] + item["processing_time"]:
            raise RuntimeError("rust_oracle_schedule_duration_mismatch")

    return {
        "schema_version": "external-fjsp-sdst-cp-sat-crosscheck/0.2",
        "upstream_commit": UPSTREAM_COMMIT,
        "upstream_model_path": "solution_methods/cp_sat/models/FJSPSDSTmodel.py",
        "upstream_model_sha256": hashlib.sha256(model_path.read_bytes()).hexdigest(),
        "input_sha256": hashlib.sha256(instance_path.read_bytes()).hexdigest(),
        "solver": "OR-Tools CP-SAT",
        "ortools_version": ortools.__version__,
        "solver_status": status_name,
        "objective": objective,
        "rust_oracle_makespan": oracle["optimal_makespan"],
        "rust_oracle_makespan_matched": objective == oracle["optimal_makespan"],
        "rust_oracle_machine_assignment_upstream_ids": oracle["optimal_machine_assignment_upstream_ids"],
        "rust_oracle_machine_sequences_operation_ids": oracle["optimal_machine_sequences_operation_ids"],
        "reference_machine_sequences_operation_ids": machine_sequences,
        "reference_schedule": schedule,
        "claim_ceiling": CLAIM_CEILING,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--instance", required=True)
    parser.add_argument("--upstream-model", required=True)
    parser.add_argument("--rust-report", required=True)
    parser.add_argument("--upstream-commit", required=True)
    args = parser.parse_args()
    result = run_crosscheck(args.instance, args.upstream_model, args.rust_report, args.upstream_commit)
    print(json.dumps(result, indent=2, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"upstream_cp_sat_crosscheck_error: {error}", file=sys.stderr)
        raise
