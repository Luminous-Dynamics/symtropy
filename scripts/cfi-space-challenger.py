#!/usr/bin/env python3
"""Independent standard-library challenger for the CFI-Space v0.1 Rust scorer.

This is test-only research tooling, not production benchmark infrastructure.
It intentionally reimplements the contract joins and count metrics in Python.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

MAX_INPUT_BYTES = 16 * 1024 * 1024
MAX_EPISODES = 100_000
MAX_FRAMES = 1_000_000

DISPOSITIONS = {
    "qualified", "abstain", "out_of_scope", "disagreement",
    "dependence_unknown", "missing_evidence",
}
FAULT_FAMILIES = [
    "rgb_local", "event_local", "timing", "extrinsics", "common_cause",
    "translation", "dynamics", "observability", "integrity",
]
SPLITS = {"development", "calibration", "confirmatory", "blind_holdout"}
VARIANTS = {"clean", "single_fault", "interaction"}
HEX64 = re.compile(r"^[a-f0-9]{64}$")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def exact_keys(value: dict[str, Any], required: set[str], where: str) -> None:
    require(isinstance(value, dict), f"{where} must be an object")
    missing = required - value.keys()
    extra = value.keys() - required
    require(not missing, f"{where} missing fields: {sorted(missing)}")
    require(not extra, f"{where} has unknown fields: {sorted(extra)}")


def read_json(path: Path) -> Any:
    size = path.stat().st_size
    require(size <= MAX_INPUT_BYTES, f"{path} exceeds {MAX_INPUT_BYTES} bytes")
    return json.loads(path.read_text(encoding="utf-8"))


def is_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def is_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def validate_candidate(doc: dict[str, Any]) -> dict[str, dict[str, Any]]:
    exact_keys(doc, {"schema_version", "benchmark_id", "episodes"}, "manifest")
    require(doc["schema_version"] == 1, "unsupported manifest schema version")
    require(is_string(doc["benchmark_id"]), "manifest benchmark_id is empty")
    episodes = doc["episodes"]
    require(isinstance(episodes, list) and 0 < len(episodes) <= MAX_EPISODES,
            "manifest episode count outside permitted range")
    by_id: dict[str, dict[str, Any]] = {}
    for episode in episodes:
        exact_keys(episode, {
            "episode_id", "input_bundle_sha256", "frame_budget",
            "expected_frame_count", "first_sequence", "horizon_us", "frames",
        }, "candidate episode")
        eid = episode["episode_id"]
        require(is_string(eid), "empty episode_id")
        require(eid not in by_id, f"duplicate episode_id {eid}")
        require(isinstance(episode["input_bundle_sha256"], str)
                and HEX64.fullmatch(episode["input_bundle_sha256"]) is not None,
                f"malformed input digest for {eid}")
        budget = episode["frame_budget"]
        expected = episode["expected_frame_count"]
        first = episode["first_sequence"]
        horizon = episode["horizon_us"]
        frames = episode["frames"]
        require(is_int(budget) and budget > 0, f"invalid frame budget for {eid}")
        require(is_int(expected) and 0 < expected <= budget,
                f"invalid expected frame count for {eid}")
        require(is_int(first) and first >= 0, f"invalid first sequence for {eid}")
        require(is_int(horizon) and horizon > 0, f"invalid horizon for {eid}")
        require(isinstance(frames, list) and len(frames) == expected
                and len(frames) <= MAX_FRAMES, f"frame count mismatch for {eid}")
        frame_ids: set[str] = set()
        first_ts: int | None = None
        previous_ts: int | None = None
        for offset, frame in enumerate(frames):
            exact_keys(frame, {"frame_id", "sequence", "timestamp_us", "modalities"},
                       "candidate frame")
            fid = frame["frame_id"]
            sequence = frame["sequence"]
            timestamp = frame["timestamp_us"]
            modalities = frame["modalities"]
            require(is_string(fid) and fid not in frame_ids,
                    f"empty or duplicate frame_id in {eid}")
            frame_ids.add(fid)
            require(is_int(sequence) and sequence == first + offset,
                    f"sequence discontinuity in {eid}")
            require(is_int(timestamp) and timestamp >= 0,
                    f"invalid timestamp in {eid}")
            require(isinstance(modalities, list) and modalities
                    and len(modalities) == len(set(modalities)),
                    f"empty or repeated modalities in {eid}")
            require(all(x in {"rgb", "event", "depth", "infrared", "lidar"}
                        for x in modalities), f"unknown modality in {eid}")
            if previous_ts is not None:
                require(timestamp > previous_ts, f"timestamps not increasing in {eid}")
            else:
                first_ts = timestamp
            previous_ts = timestamp
        require(first_ts is not None and previous_ts is not None,
                f"no timestamps in {eid}")
        require(previous_ts - first_ts <= horizon, f"horizon exceeded in {eid}")
        by_id[eid] = episode
    return by_id


def validate_ledger(doc: dict[str, Any]) -> dict[str, dict[str, Any]]:
    exact_keys(doc, {"schema_version", "benchmark_id", "episodes"}, "ledger")
    require(doc["schema_version"] == 1, "unsupported ledger schema version")
    require(is_string(doc["benchmark_id"]), "ledger benchmark_id is empty")
    rows = doc["episodes"]
    require(isinstance(rows, list) and 0 < len(rows) <= MAX_EPISODES,
            "ledger episode count outside permitted range")
    by_id: dict[str, dict[str, Any]] = {}
    group_keys: dict[str, tuple[Any, ...]] = {}
    split_maps: dict[str, dict[str, str]] = {
        "target": {}, "session": {}, "trajectory": {}, "rig": {},
    }
    for row in rows:
        exact_keys(row, {
            "episode_id", "parent_episode_id", "counterfactual_group_id",
            "variant_kind", "split", "independent_unit", "fault_ids",
            "fault_families", "critical", "expected_disposition",
        }, "ledger episode")
        eid = row["episode_id"]
        require(is_string(eid) and eid not in by_id,
                f"empty or duplicate ledger episode_id {eid!r}")
        for field in ("parent_episode_id", "counterfactual_group_id"):
            require(is_string(row[field]), f"empty {field} for {eid}")
        require(row["variant_kind"] in VARIANTS, f"unknown variant kind for {eid}")
        require(row["split"] in SPLITS, f"unknown split for {eid}")
        require(row["expected_disposition"] in DISPOSITIONS,
                f"unknown expected disposition for {eid}")
        require(type(row["critical"]) is bool, f"critical must be boolean for {eid}")
        unit = row["independent_unit"]
        exact_keys(unit, {"target_id", "session_id", "trajectory_id", "sensor_rig_id"},
                   "independent unit")
        unit_values = (
            unit["target_id"], unit["session_id"], unit["trajectory_id"],
            unit["sensor_rig_id"],
        )
        require(all(is_string(value) for value in unit_values),
                f"empty independent-unit field for {eid}")
        fault_ids = row["fault_ids"]
        families = row["fault_families"]
        require(isinstance(fault_ids, list) and all(is_string(x) for x in fault_ids)
                and len(fault_ids) == len(set(fault_ids)),
                f"duplicate or empty fault IDs for {eid}")
        require(isinstance(families, list)
                and all(x in FAULT_FAMILIES for x in families)
                and len(families) == len(set(families)),
                f"duplicate or unknown fault families for {eid}")
        disposition = row["expected_disposition"]
        if row["variant_kind"] == "clean":
            require(not fault_ids and not families and disposition == "qualified",
                    f"clean episode contract violated for {eid}")
        elif row["variant_kind"] == "single_fault":
            require(len(fault_ids) == 1 and len(families) == 1
                    and disposition != "qualified",
                    f"single-fault contract violated for {eid}")
        else:
            require(len(fault_ids) >= 2 and families and disposition != "qualified",
                    f"interaction contract violated for {eid}")
        group = row["counterfactual_group_id"]
        key = (row["parent_episode_id"], row["split"], *unit_values)
        if group in group_keys:
            require(group_keys[group] == key,
                    f"counterfactual group {group} crosses parent/split/unit")
        else:
            group_keys[group] = key
        for name, value in zip(split_maps, unit_values, strict=True):
            previous = split_maps[name].setdefault(value, row["split"])
            require(previous == row["split"],
                    f"{name} {value} leaks across splits")
        by_id[eid] = row
    return by_id


def score(manifest: dict[str, Any], ledger: dict[str, Any],
          prediction_doc: dict[str, Any]) -> dict[str, Any]:
    candidate_by_id = validate_candidate(manifest)
    truth_by_id = validate_ledger(ledger)
    exact_keys(prediction_doc, {"schema_version", "benchmark_id", "predictions"},
               "predictions")
    require(prediction_doc["schema_version"] == 1,
            "unsupported predictions schema version")
    benchmark_id = manifest["benchmark_id"]
    require(benchmark_id == ledger["benchmark_id"] == prediction_doc["benchmark_id"],
            "benchmark_id mismatch")
    rows = prediction_doc["predictions"]
    require(isinstance(rows, list) and len(rows) == len(candidate_by_id),
            "prediction count mismatch")

    prediction_by_id: dict[str, dict[str, Any]] = {}
    for row in rows:
        exact_keys(row, {
            "episode_id", "input_bundle_sha256", "disposition",
            "detected_fault_families",
        }, "prediction")
        eid = row["episode_id"]
        require(is_string(eid) and eid not in prediction_by_id,
                f"empty or duplicate prediction identity {eid!r}")
        require(row["disposition"] in DISPOSITIONS, f"unknown disposition for {eid}")
        detected = row["detected_fault_families"]
        require(isinstance(detected, list)
                and all(family in FAULT_FAMILIES for family in detected)
                and len(detected) == len(set(detected)),
                f"duplicate or unknown predicted family for {eid}")
        prediction_by_id[eid] = row

    ids = set(candidate_by_id)
    require(ids == set(truth_by_id), "manifest / ledger episode identities differ")
    require(ids == set(prediction_by_id), "manifest / prediction episode identities differ")

    clean_episodes = fault_episodes = 0
    critical_fault_episodes = critical_correctly_contained_faults = 0
    critical_unsafe_accepts = 0
    clean_qualified = false_abstentions = correctly_contained_faults = 0
    unsafe_accepts = exact_disposition_matches = false_positive_family_labels = 0
    family_counts = {family: [0, 0] for family in FAULT_FAMILIES}

    for eid, episode in candidate_by_id.items():
        truth = truth_by_id[eid]
        prediction = prediction_by_id[eid]
        digest = episode["input_bundle_sha256"]
        require(prediction["input_bundle_sha256"] == digest
                and HEX64.fullmatch(prediction["input_bundle_sha256"]) is not None,
                f"prediction for {eid} is not bound to input digest")
        expected_families = set(truth["fault_families"])
        predicted_families = set(prediction["detected_fault_families"])
        false_positive_family_labels += len(predicted_families - expected_families)
        for family in expected_families:
            family_counts[family][0] += 1
            family_counts[family][1] += family in predicted_families

        expected = truth["expected_disposition"]
        actual = prediction["disposition"]
        exact_disposition_matches += expected == actual
        if expected == "qualified":
            clean_episodes += 1
            if actual == "qualified":
                clean_qualified += 1
            else:
                false_abstentions += 1
        else:
            fault_episodes += 1
            critical_fault_episodes += truth["critical"]
            if actual == "qualified":
                unsafe_accepts += 1
                critical_unsafe_accepts += truth["critical"]
            else:
                correctly_contained_faults += 1
                critical_correctly_contained_faults += truth["critical"]
        require(episode["episode_id"] == eid, "internal candidate identity mismatch")

    independent_units = {
        tuple(row["independent_unit"][key]
              for key in ("target_id", "session_id", "trajectory_id", "sensor_rig_id"))
        for row in truth_by_id.values()
    }
    groups = {row["counterfactual_group_id"] for row in truth_by_id.values()}
    family_detection = [
        {"family": family, "episodes": values[0], "detected": values[1]}
        for family, values in family_counts.items() if values[0] > 0
    ]
    return {
        "benchmark_id": benchmark_id,
        "episodes": len(candidate_by_id),
        "independent_units": len(independent_units),
        "counterfactual_groups": len(groups),
        "clean_episodes": clean_episodes,
        "fault_episodes": fault_episodes,
        "critical_fault_episodes": critical_fault_episodes,
        "critical_correctly_contained_faults": critical_correctly_contained_faults,
        "critical_unsafe_accepts": critical_unsafe_accepts,
        "clean_qualified": clean_qualified,
        "false_abstentions": false_abstentions,
        "correctly_contained_faults": correctly_contained_faults,
        "unsafe_accepts": unsafe_accepts,
        "exact_disposition_matches": exact_disposition_matches,
        "false_positive_family_labels": false_positive_family_labels,
        "family_detection": family_detection,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--ledger", type=Path, required=True)
    parser.add_argument("--predictions", type=Path, required=True)
    parser.add_argument("--expect-unsafe", type=int, required=True)
    args = parser.parse_args()
    try:
        result = score(read_json(args.manifest), read_json(args.ledger),
                       read_json(args.predictions))
        print(json.dumps(result, indent=2))
        require(result["unsafe_accepts"] == args.expect_unsafe,
                f"unsafe_accepts expected {args.expect_unsafe}, got {result['unsafe_accepts']}")
        return 0
    except (OSError, json.JSONDecodeError, ValueError, TypeError, KeyError) as error:
        print(f"cfi-space-challenger: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
