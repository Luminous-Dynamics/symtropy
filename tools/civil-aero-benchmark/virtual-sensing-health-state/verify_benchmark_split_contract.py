#!/usr/bin/env python3
"""Independent standard-library verifier for SHM benchmark split leakage."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "data" / "benchmark-split-contract-v0.1.json"
CLAIM = "synthetic_partition_integrity_only_no_dataset_or_model_performance_claim"
SPLITS = ("train", "validation", "test")


class AuditError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def validate_policy(policy: dict[str, Any]) -> None:
    require(policy["required_splits"] == list(SPLITS), "required split set changed")
    for key in (
        "every_sample_has_unique_sample_id",
        "every_sample_has_unique_source_record_id_within_track",
        "every_sample_has_unique_acquisition_window_id_within_track",
        "held_out_group_must_be_disjoint_across_splits",
        "group_holdout_requires_one_nonempty_holdout_axis",
        "temporal_forward_requires_fixed_article_and_installation",
        "temporal_forward_requires_purge_gap_at_least_feature_window_plus_preprocessing_memory",
        "no_random_window_level_split_claim",
    ):
        require(policy[key] is True, f"split policy weakened: {key}")


def axis_value(record: dict[str, Any], axis: str) -> str | None:
    if axis not in (
        "specimen_id",
        "thermal_regime_id",
        "installation_generation",
        "campaign_id",
        "configuration_id",
    ):
        return None
    return record[axis]


def validate_group_holdout(track: dict[str, Any]) -> list[str]:
    specification = track.get("holdout_axis")
    require(isinstance(specification, str) and specification.strip(),
            "group holdout requires supported holdout axis")
    axes = specification.split("+")
    supported = {
        "specimen_id",
        "thermal_regime_id",
        "installation_generation",
        "campaign_id",
        "configuration_id",
    }
    require(
        all(axis in supported for axis in axes) and len(axes) == len(set(axes)),
        "unsupported, empty or duplicate group holdout axis",
    )
    require(track.get("purge_policy") is None,
            "group-holdout track must not carry a temporal purge policy")
    checks = [
        "SAMPLE_ID_UNIQUE",
        "SOURCE_RECORD_ID_UNIQUE",
        "ACQUISITION_WINDOW_ID_UNIQUE",
    ]
    for axis in axes:
        values = {
            split: {
                axis_value(record, axis)
                for record in track["records"]
                if record["split"] == split
            }
            for split in SPLITS
        }
        require(all(values[split] for split in SPLITS),
                "required split missing from group holdout")
        for left_index, left in enumerate(SPLITS):
            for right in SPLITS[left_index + 1:]:
                require(
                    values[left].isdisjoint(values[right]),
                    f"holdout leakage on axis {axis} between {left} and {right}",
                )
        checks.append(f"GROUP_DISJOINT:{axis}")
    return checks


def validate_temporal_forward(track: dict[str, Any]) -> list[str]:
    require(track.get("holdout_axis") is None,
            "purged forward track must not declare group holdout axis")
    purge = track.get("purge_policy")
    require(isinstance(purge, dict), "temporal forward track requires purge policy")
    for key in (
        "feature_window_seconds",
        "preprocessing_memory_seconds",
        "minimum_purge_gap_seconds",
    ):
        value = purge[key]
        require(isinstance(value, (int, float)) and math.isfinite(value) and value >= 0.0,
                f"{key} must be finite and non-negative")
    required_gap = purge["feature_window_seconds"] + purge["preprocessing_memory_seconds"]
    require(purge["minimum_purge_gap_seconds"] + 1e-9 >= required_gap,
            "minimum purge gap is shorter than feature window plus preprocessing memory")

    records = track["records"]
    reference = records[0]
    for record in records:
        for key in ("specimen_id", "campaign_id", "configuration_id", "installation_generation"):
            require(record[key] == reference[key],
                    "temporal-forward track must hold article, campaign, configuration and installation fixed")
        start, end = record.get("interval_start_s"), record.get("interval_end_s")
        require(
            isinstance(start, (int, float)) and isinstance(end, (int, float))
            and math.isfinite(start) and math.isfinite(end)
            and start >= 0.0 and end > start,
            "temporal interval must be finite, non-negative and increasing",
        )

    def interval(split: str) -> tuple[float, float]:
        selected = [record for record in records if record["split"] == split]
        require(selected, f"temporal-forward split {split} is empty")
        return (
            min(record["interval_start_s"] for record in selected),
            max(record["interval_end_s"] for record in selected),
        )

    train, validation, test = (interval(split) for split in SPLITS)
    gap = purge["minimum_purge_gap_seconds"]
    require(
        validation[0] - train[1] + 1e-9 >= gap
        and test[0] - validation[1] + 1e-9 >= gap,
        "forward partitions overlap the declared preprocessing/feature purge horizon",
    )
    return [
        "SAMPLE_ID_UNIQUE",
        "SOURCE_RECORD_ID_UNIQUE",
        "ACQUISITION_WINDOW_ID_UNIQUE",
        "FIXED_ARTICLE_CONFIGURATION_INSTALLATION",
        "FORWARD_TIME_ORDER",
        "PURGE_GAP_GE_FEATURE_PLUS_PREPROCESSING_MEMORY",
    ]


def validate_track(track: dict[str, Any]) -> dict[str, Any]:
    require(isinstance(track.get("track_id"), str) and track["track_id"].strip(),
            "track identity must be non-empty")
    require(isinstance(track.get("objective"), str) and track["objective"].strip(),
            "track objective must be non-empty")
    require(track["required_splits"] == list(SPLITS), "track required split set changed")
    records = track["records"]
    require(isinstance(records, list) and len(records) >= 3,
            "at least one record per split is required")

    identity_keys = (
        "sample_id", "dataset_id", "source_record_id", "acquisition_window_id",
        "specimen_id", "campaign_id", "configuration_id", "thermal_regime_id",
        "installation_generation",
    )
    for record in records:
        require(all(isinstance(record.get(key), str) and record[key].strip()
                    for key in identity_keys),
                "record contains empty identity field")
        require(record["split"] in SPLITS, "unsupported split label")

    for key in ("sample_id", "source_record_id", "acquisition_window_id"):
        values = [record[key] for record in records]
        require(len(values) == len(set(values)),
                f"duplicate sample/source record/acquisition window ({key})")

    counts = {split: sum(record["split"] == split for record in records) for split in SPLITS}
    require(all(counts[split] > 0 for split in SPLITS),
            "every required split must be populated")
    mode = track["split_mode"]
    if mode == "group_holdout":
        checks = validate_group_holdout(track)
    elif mode == "purged_forward_chaining":
        checks = validate_temporal_forward(track)
    else:
        raise AuditError("unsupported split mode")
    return {
        "track_id": track["track_id"],
        "split_mode": mode,
        "holdout_axis": track.get("holdout_axis"),
        "sample_count": len(records),
        "split_counts": [[split, counts[split]] for split in SPLITS if counts[split]],
        "leakage_checks": checks,
        "status": "PASS_SYNTHETIC_PARTITION_INTEGRITY",
    }


def validate_fixture(data: dict[str, Any]) -> None:
    require(data.get("schema_version") == "aero-shm-benchmark-split-contract/0.1.0",
            "unsupported split-contract schema")
    require(isinstance(data.get("contract_id"), str) and data["contract_id"].strip(),
            "contract ID must be non-empty")
    require(data.get("claim_ceiling") == CLAIM, "claim ceiling changed or weakened")
    validate_policy(data["policy"])
    tracks = data["tracks"]
    require(isinstance(tracks, list) and 3 <= len(tracks) <= 32,
            "track count must be 3..=32")
    ids = [track.get("track_id") for track in tracks]
    require(len(ids) == len(set(ids)), "duplicate track_id")
    for track in tracks:
        validate_track(track)


def evaluate(data: dict[str, Any], raw: bytes) -> dict[str, Any]:
    validate_fixture(data)
    tracks = [validate_track(track) for track in data["tracks"]]
    return {
        "schema_version": "aero-shm-benchmark-split-contract-result/0.1.0",
        "contract_id": data["contract_id"],
        "source_sha256": hashlib.sha256(raw).hexdigest(),
        "source_bytes": len(raw),
        "track_count": len(tracks),
        "sample_record_count": sum(track["sample_count"] for track in tracks),
        "tracks": tracks,
        "claim_ceiling": CLAIM,
    }


def compare(actual: Any, expected: Any, path: str = "root") -> None:
    if isinstance(expected, float):
        require(
            isinstance(actual, (int, float))
            and math.isclose(float(actual), expected, rel_tol=1e-10, abs_tol=1e-12),
            f"{path}: {actual!r} != {expected!r}",
        )
    elif isinstance(expected, dict):
        require(isinstance(actual, dict) and set(actual) == set(expected),
                f"{path}: object keys differ")
        for key in expected:
            compare(actual[key], expected[key], f"{path}.{key}")
    elif isinstance(expected, list):
        require(isinstance(actual, list) and len(actual) == len(expected),
                f"{path}: list length differs")
        for index, (left, right) in enumerate(zip(actual, expected)):
            compare(left, right, f"{path}[{index}]")
    else:
        require(actual == expected, f"{path}: {actual!r} != {expected!r}")


def negative_controls(data: dict[str, Any]) -> None:
    duplicate = copy.deepcopy(data)
    track = next(item for item in duplicate["tracks"]
                 if item["track_id"] == "SPECIMEN_HOLDOUT")
    track["records"][4]["acquisition_window_id"] = track["records"][0]["acquisition_window_id"]
    try:
        validate_fixture(duplicate)
    except AuditError as error:
        require("duplicate" in str(error), "duplicate-window negative control failed for wrong reason")
    else:
        raise AuditError("negative control failed: duplicate acquisition window accepted")

    overlap = copy.deepcopy(data)
    track = next(item for item in overlap["tracks"]
                 if item["track_id"] == "SPECIMEN_HOLDOUT")
    track["records"][4]["specimen_id"] = track["records"][0]["specimen_id"]
    try:
        validate_fixture(overlap)
    except AuditError as error:
        require("holdout leakage" in str(error), "group-overlap negative control failed for wrong reason")
    else:
        raise AuditError("negative control failed: specimen leakage accepted")

    compound = copy.deepcopy(data)
    track = next(item for item in compound["tracks"]
                 if item["track_id"] == "COMPOUND_ARTICLE_CAMPAIGN_HOLDOUT")
    track["records"][2]["configuration_id"] = track["records"][0]["configuration_id"]
    try:
        validate_fixture(compound)
    except AuditError as error:
        require("holdout leakage on axis configuration_id" in str(error),
                "compound-axis negative control failed for wrong reason")
    else:
        raise AuditError("negative control failed: compound configuration leakage accepted")

    purge = copy.deepcopy(data)
    track = next(item for item in purge["tracks"]
                 if item["track_id"] == "TEMPORAL_FORWARD")
    next(record for record in track["records"]
         if record["split"] == "validation")["interval_start_s"] = 4000.0
    try:
        validate_fixture(purge)
    except AuditError as error:
        require("purge horizon" in str(error), "purge negative control failed for wrong reason")
    else:
        raise AuditError("negative control failed: insufficient purge gap accepted")

    print("negative_control=pass case=duplicate_acquisition_window")
    print("negative_control=pass case=held_out_specimen_overlap")
    print("negative_control=pass case=compound_configuration_campaign_specimen_leak")
    print("negative_control=pass case=insufficient_temporal_purge_gap")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path, required=True)
    args = parser.parse_args()
    try:
        raw = FIXTURE.read_bytes()
        data = json.loads(raw)
        expected = evaluate(data, raw)
        require(expected["track_count"] == 5, "frozen split-track count changed")
        require(expected["sample_record_count"] == 17, "frozen sample-record count changed")
        actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
        compare(actual, expected)
        negative_controls(data)
        print(
            "aero_shm_benchmark_split_contract=pass tracks=5 samples=17 "
            "axes=specimen+thermal+installation+compound+forward_time rust_python=pass"
        )
        return 0
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"benchmark_split_contract_error={error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
