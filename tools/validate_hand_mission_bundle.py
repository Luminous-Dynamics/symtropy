#!/usr/bin/env python3
"""Schema and semantic verifier for the hand-mission evidence bundle.

This is benchmark validation tooling, not production robotics software.
"""
from __future__ import annotations

import copy
import hashlib
import json
import math
from datetime import datetime
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1] / "benchmarks" / "hand-mission-qualification" / "v0.1"
SCHEMA_PATH = ROOT / "hand_mission_bundle.schema.json"
EXAMPLE_PATH = ROOT / "examples" / "pomdar_repeated_mission_illustrative.json"
TOLERANCE_S = 1.0e-9


def canonical_digest(value: Any) -> str:
    encoded = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def unique(values: list[str]) -> bool:
    return len(values) == len(set(values))


def semantic_errors(bundle: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    profile = bundle["benchmark_profile"]
    boundary = bundle["system_boundary"]
    clock = bundle["mission_clock"]
    attempts = bundle["attempts"]
    timeline = bundle["timeline"]
    summary = bundle["run_summary"]
    tasks = bundle["task_catalog"]

    task_ids = [task["task_id"] for task in tasks]
    attempt_ids = [attempt["attempt_id"] for attempt in attempts]
    event_ids = [event["event_id"] for event in timeline]
    evidence_ids = [item["evidence_id"] for item in bundle["evidence"]]

    if not unique(task_ids):
        errors.append("task_catalog task_id values must be unique")
    if not unique(attempt_ids):
        errors.append("attempt_id values must be unique")
    if not unique(event_ids):
        errors.append("timeline event_id values must be unique")
    if not unique(evidence_ids):
        errors.append("evidence_id values must be unique")

    valid_task_ids = set(task_ids)
    if any(attempt["task_id"] not in valid_task_ids for attempt in attempts):
        errors.append("every attempt must reference a task in task_catalog")

    planned_sequence = profile["planned_task_sequence"]
    if any(task_id not in valid_task_ids for task_id in planned_sequence):
        errors.append("planned_task_sequence contains an unknown task_id")

    actual_sequence = [
        attempt["task_id"]
        for attempt in sorted(attempts, key=lambda item: item["start_elapsed_s"])
    ]
    if actual_sequence != planned_sequence:
        errors.append("attempt order must exactly match benchmark_profile.planned_task_sequence")

    expected_task_digest = canonical_digest(sorted(tasks, key=lambda item: item["task_id"]))
    if profile["task_set_digest"] != expected_task_digest:
        errors.append("task_set_digest does not match canonical sorted task_catalog")
    expected_mix_digest = canonical_digest(planned_sequence)
    if profile["task_mix_digest"] != expected_mix_digest:
        errors.append("task_mix_digest does not match canonical planned_task_sequence")

    elapsed = clock["elapsed_s"]
    if not math.isclose(summary["mission_elapsed_s"], elapsed, rel_tol=0.0, abs_tol=TOLERANCE_S):
        errors.append("run_summary.mission_elapsed_s must equal mission_clock.elapsed_s")

    try:
        start = datetime.fromisoformat(clock["start_utc"].replace("Z", "+00:00"))
        end = datetime.fromisoformat(clock["end_utc"].replace("Z", "+00:00"))
        wall_elapsed = (end - start).total_seconds()
        if not math.isclose(wall_elapsed, elapsed, rel_tol=0.0, abs_tol=TOLERANCE_S):
            errors.append("mission_clock.elapsed_s must equal end_utc minus start_utc")
    except (ValueError, TypeError):
        errors.append("mission_clock timestamps must be valid offset-aware ISO-8601 values")

    outcomes = ["accepted", "failed", "aborted", "censored", "excluded"]
    outcome_counts = {
        outcome: sum(attempt["outcome"] == outcome for attempt in attempts)
        for outcome in outcomes
    }
    if summary["planned_attempts"] != len(attempts):
        errors.append("planned_attempts must equal the complete attempts array length")
    summary_fields = {
        "accepted": "accepted_task_completions",
        "failed": "failed_attempts",
        "aborted": "aborted_attempts",
        "censored": "censored_attempts",
        "excluded": "excluded_attempts",
    }
    for outcome, field in summary_fields.items():
        if summary[field] != outcome_counts[outcome]:
            errors.append(f"run_summary.{field} does not match attempt outcomes")

    expected_throughput = outcome_counts["accepted"] / (elapsed / 3600.0)
    if not math.isclose(
        summary["sustained_mission_throughput_per_h"],
        expected_throughput,
        rel_tol=1.0e-9,
        abs_tol=1.0e-9,
    ):
        errors.append("sustained_mission_throughput_per_h must be recomputed from accepted attempts and full elapsed time")

    ordered_timeline = sorted(timeline, key=lambda event: event["start_elapsed_s"])
    cursor = 0.0
    for event in ordered_timeline:
        event_start = event["start_elapsed_s"]
        event_end = event["end_elapsed_s"]
        if event_start < -TOLERANCE_S or event_end <= event_start:
            errors.append(f"timeline event {event['event_id']} has an invalid interval")
        if not math.isclose(event_start, cursor, rel_tol=0.0, abs_tol=TOLERANCE_S):
            errors.append(f"timeline has a gap or overlap before event {event['event_id']}")
        if event_end > elapsed + TOLERANCE_S:
            errors.append(f"timeline event {event['event_id']} exceeds mission clock")
        cursor = event_end
    if not math.isclose(cursor, elapsed, rel_tol=0.0, abs_tol=TOLERANCE_S):
        errors.append("timeline must cover the full mission interval [0, elapsed_s]")

    task_events = [event for event in timeline if event["event_type"] == "task_execution"]
    if len(task_events) != len(attempts):
        errors.append("there must be exactly one task_execution event per attempt")
    events_by_attempt = {
        event["attempt_id"]: event
        for event in task_events
        if event["attempt_id"] is not None
    }
    for attempt in attempts:
        event = events_by_attempt.get(attempt["attempt_id"])
        if event is None:
            errors.append(f"attempt {attempt['attempt_id']} has no task_execution event")
            continue
        if (
            event["start_elapsed_s"] != attempt["start_elapsed_s"]
            or event["end_elapsed_s"] != attempt["end_elapsed_s"]
        ):
            errors.append(f"task_execution interval does not match attempt {attempt['attempt_id']}")

    sorted_attempts = sorted(attempts, key=lambda item: item["start_elapsed_s"])
    previous_end = 0.0
    for attempt in sorted_attempts:
        if attempt["end_elapsed_s"] <= attempt["start_elapsed_s"]:
            errors.append(f"attempt {attempt['attempt_id']} has a non-positive duration")
        if attempt["start_elapsed_s"] < previous_end - TOLERANCE_S:
            errors.append(f"attempt {attempt['attempt_id']} overlaps a preceding attempt")
        if attempt["end_elapsed_s"] > elapsed + TOLERANCE_S:
            errors.append(f"attempt {attempt['attempt_id']} exceeds mission clock")
        previous_end = attempt["end_elapsed_s"]

    unknown_power = boundary["unknown_power_components"]
    complete_boundaries = {"actuators_only", "hand_including_drivers", "arm_and_hand", "full_cell"}
    if unknown_power and boundary["power_measurement_boundary"] in complete_boundaries:
        errors.append("unknown_power_components cannot coexist with a declared complete power_measurement_boundary")

    known_evidence_ids = set(evidence_ids)
    for observation in bundle["observations"]:
        missing = set(observation["evidence_refs"]) - known_evidence_ids
        if missing:
            errors.append(f"observation {observation['metric_id']} references unknown evidence IDs: {sorted(missing)}")

    return errors


def validate_bundle(bundle: dict[str, Any], validator: Draft202012Validator) -> list[str]:
    schema_errors = sorted(
        validator.iter_errors(bundle),
        key=lambda error: tuple(str(part) for part in error.path),
    )
    messages = [
        "schema " + "/".join(str(part) for part in error.path) + ": " + error.message
        for error in schema_errors
    ]
    if not schema_errors:
        messages.extend(semantic_errors(bundle))
    return messages


def expect_rejected(
    name: str,
    original: dict[str, Any],
    validator: Draft202012Validator,
    mutate,
) -> None:
    candidate = copy.deepcopy(original)
    mutate(candidate)
    errors = validate_bundle(candidate, validator)
    if not errors:
        raise SystemExit(f"negative control unexpectedly accepted: {name}")
    print(f"rejected negative control: {name}")


def main() -> None:
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    example = json.loads(EXAMPLE_PATH.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())

    errors = validate_bundle(example, validator)
    if errors:
        raise SystemExit("\n".join(errors))
    print("positive fixture: schema and semantic validation passed")

    expect_rejected(
        "missing attempt denominator",
        example,
        validator,
        lambda item: item["run_summary"].pop("planned_attempts"),
    )
    expect_rejected(
        "cooldown omitted from mission timeline",
        example,
        validator,
        lambda item: item["timeline"].remove(
            next(event for event in item["timeline"] if event["event_type"] == "cooldown")
        ),
    )
    expect_rejected(
        "throughput excludes or miscounts elapsed mission time",
        example,
        validator,
        lambda item: item["run_summary"].update(sustained_mission_throughput_per_h=144.0),
    )
    expect_rejected(
        "task catalog changed without digest update",
        example,
        validator,
        lambda item: item["task_catalog"][0].update(acceptance_criteria="changed task"),
    )
    expect_rejected(
        "planned task mix changed without digest update",
        example,
        validator,
        lambda item: item["benchmark_profile"].update(
            planned_task_sequence=["horizontal_pinch", "vertical_sphere", "vertical_sphere"]
        ),
    )
    expect_rejected(
        "unknown power components advertised as complete boundary",
        example,
        validator,
        lambda item: item["system_boundary"].update(power_measurement_boundary="hand_including_drivers"),
    )
    expect_rejected(
        "attempt and task event timestamps disagree",
        example,
        validator,
        lambda item: item["timeline"][0].update(end_elapsed_s=19.0),
    )
    expect_rejected(
        "duplicate attempt identifier",
        example,
        validator,
        lambda item: item["attempts"][1].update(attempt_id="attempt-001"),
    )


if __name__ == "__main__":
    main()
