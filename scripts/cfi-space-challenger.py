#!/usr/bin/env python3
"""Independent standard-library challenger for the CFI-Space v0.2 Rust scorer.

This is test-only research tooling, not production benchmark infrastructure.
It intentionally reimplements the contract joins and count metrics in Python.
"""
from __future__ import annotations

import argparse
import hashlib
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
GENERALIZATION_ORDER = (
    "seen_target", "unseen_target", "unseen_target_family",
    "unseen_renderer", "unseen_sensor_rig",
)
GENERALIZATION = set(GENERALIZATION_ORDER)
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


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def read_json(path: Path) -> Any:
    size = path.stat().st_size
    require(size <= MAX_INPUT_BYTES, f"{path} exceeds {MAX_INPUT_BYTES} bytes")
    raw = path.read_bytes()
    require(len(raw) <= MAX_INPUT_BYTES,
            f"{path} exceeds {MAX_INPUT_BYTES} bytes after read")
    return json.loads(raw.decode("utf-8"), object_pairs_hook=unique_object)


def is_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def is_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def validate_vec3(value: Any, where: str) -> list[float]:
    require(isinstance(value, list) and len(value) == 3
            and all(is_number(x) for x in value),
            f"{where} must contain three finite numbers")
    result = [float(x) for x in value]
    require(all(__import__("math").isfinite(x) for x in result),
            f"{where} contains a non-finite number")
    return result


def validate_quaternion(value: Any, where: str) -> list[float]:
    import math
    require(isinstance(value, list) and len(value) == 4
            and all(is_number(x) for x in value),
            f"{where} must contain four finite numbers")
    result = [float(x) for x in value]
    require(all(math.isfinite(x) for x in result),
            f"{where} contains a non-finite number")
    norm = math.sqrt(sum(x * x for x in result))
    require(math.isfinite(norm) and abs(norm - 1.0) <= 1e-3,
            f"{where} must be a unit quaternion (norm tolerance 1e-3)")
    return result


def quaternion_distance(left: list[float], right: list[float]) -> float:
    import math
    left_norm = math.sqrt(sum(x * x for x in left))
    right_norm = math.sqrt(sum(x * x for x in right))
    dot = sum(a * b for a, b in zip(left, right, strict=True)) / (left_norm * right_norm)
    return 2.0 * math.acos(max(0.0, min(1.0, abs(dot))))


def quantile(values: list[float], probability: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    position = probability * (len(ordered) - 1)
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def validate_candidate(doc: dict[str, Any], bundle_root: Path) -> dict[str, dict[str, Any]]:
    exact_keys(doc, {"schema_version", "benchmark_id", "episodes"}, "manifest")
    require(is_int(doc["schema_version"]) and doc["schema_version"] == 1,
            "unsupported manifest schema version")
    require(is_string(doc["benchmark_id"]), "manifest benchmark_id is empty")
    episodes = doc["episodes"]
    require(isinstance(episodes, list) and 0 < len(episodes) <= MAX_EPISODES,
            "manifest episode count outside permitted range")
    by_id: dict[str, dict[str, Any]] = {}
    for episode in episodes:
        exact_keys(episode, {
            "episode_id", "input_bundle_path", "input_bundle_sha256", "frame_budget",
            "expected_frame_count", "first_sequence", "horizon_us", "frames",
        }, "candidate episode")
        eid = episode["episode_id"]
        require(is_string(eid), "empty episode_id")
        require(eid not in by_id, f"duplicate episode_id {eid}")
        require(is_string(episode["input_bundle_path"]),
                f"empty input bundle path for {eid}")
        relative = Path(episode["input_bundle_path"])
        require(not relative.is_absolute() and relative.parts
                and all(part not in {"", ".", ".."} for part in relative.parts),
                f"unsafe input bundle path for {eid}")
        root = bundle_root.resolve(strict=True)
        resolved = (root / relative).resolve(strict=True)
        require(resolved.is_relative_to(root),
                f"input bundle path escapes declared root for {eid}")
        require(resolved.is_file() and resolved.stat().st_size <= MAX_INPUT_BYTES,
                f"input bundle is not a regular file or exceeds byte limit for {eid}")
        bundle_bytes = resolved.read_bytes()
        require(len(bundle_bytes) <= MAX_INPUT_BYTES,
                f"input bundle exceeds byte limit after read for {eid}")
        actual_digest = hashlib.sha256(bundle_bytes).hexdigest()
        require(actual_digest == episode["input_bundle_sha256"],
                f"source bundle SHA-256 mismatch for {eid}")
        require(HEX64.fullmatch(episode["input_bundle_sha256"]) is not None,
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

        try:
            bundle_lines = bundle_bytes.decode("utf-8").splitlines()
        except UnicodeDecodeError as error:
            raise ValueError(f"source bundle is not UTF-8 for {eid}: {error}") from error
        require(len(bundle_lines) == 3 + len(frames),
                f"source bundle record count mismatch for {eid}")
        expected_header = [
            "CFI_SPACE_SYNTHETIC_NON_IMAGE_PAYLOAD_V1",
            f"episode={eid}",
            f"frame_budget={budget};expected_frame_count={expected};"
            f"first_sequence={first};horizon_us={horizon}",
        ]
        require(bundle_lines[:3] == expected_header,
                f"source bundle header does not match manifest for {eid}")
        for offset, frame in enumerate(frames):
            modalities_text = ",".join(frame["modalities"])
            prefix = (
                f"frame={offset};frame_id={frame['frame_id']};"
                f"sequence={frame['sequence']};timestamp_us={frame['timestamp_us']};"
                f"modalities={modalities_text};payload="
            )
            row = bundle_lines[3 + offset]
            require(row.startswith(prefix) and len(row) > len(prefix),
                    f"source bundle frame metadata/payload mismatch for {eid} frame {offset}")
        by_id[eid] = episode
    return by_id


def validate_ledger(doc: dict[str, Any]) -> dict[str, dict[str, Any]]:
    exact_keys(doc, {"schema_version", "benchmark_id", "episodes"}, "ledger")
    require(is_int(doc["schema_version"]) and doc["schema_version"] == 2,
            "unsupported ledger schema version")
    require(is_string(doc["benchmark_id"]), "ledger benchmark_id is empty")
    rows = doc["episodes"]
    require(isinstance(rows, list) and 0 < len(rows) <= MAX_EPISODES,
            "ledger episode count outside permitted range")
    by_id: dict[str, dict[str, Any]] = {}
    group_keys: dict[str, tuple[Any, ...]] = {}
    group_variants: dict[str, list[int]] = {}
    split_maps: dict[str, dict[str, str]] = {
        "target": {}, "session": {}, "trajectory": {}, "rig": {},
    }
    for row in rows:
        exact_keys(row, {
            "episode_id", "parent_episode_id", "counterfactual_group_id",
            "variant_kind", "split", "generalization", "independent_unit",
            "fault_ids", "fault_families", "critical", "expected_disposition",
            "ground_truth_pose",
        }, "ledger episode")
        eid = row["episode_id"]
        require(is_string(eid) and eid not in by_id,
                f"empty or duplicate ledger episode_id {eid!r}")
        for field in ("parent_episode_id", "counterfactual_group_id"):
            require(is_string(row[field]), f"empty {field} for {eid}")
        require(row["variant_kind"] in VARIANTS, f"unknown variant kind for {eid}")
        require(row["split"] in SPLITS, f"unknown split for {eid}")
        require(row["generalization"] in GENERALIZATION,
                f"unknown generalization stratum for {eid}")
        require(row["expected_disposition"] in DISPOSITIONS,
                f"unknown expected disposition for {eid}")
        require(type(row["critical"]) is bool, f"critical must be boolean for {eid}")
        pose = row["ground_truth_pose"]
        exact_keys(pose, {
            "coordinate_frame", "transform_direction", "translation_m",
            "orientation_xyzw", "annotation_sha256", "label_audit_sha256",
        }, f"ground-truth pose {eid}")
        require(is_string(pose["coordinate_frame"])
                and is_string(pose["transform_direction"]),
                f"ground-truth pose convention fields are empty for {eid}")
        validate_vec3(pose["translation_m"], f"ground-truth translation {eid}")
        validate_quaternion(pose["orientation_xyzw"], f"ground-truth quaternion {eid}")
        require(HEX64.fullmatch(pose["annotation_sha256"]) is not None
                and HEX64.fullmatch(pose["label_audit_sha256"]) is not None,
                f"ground-truth annotation digest malformed for {eid}")
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
        counts = group_variants.setdefault(group, [0, 0])
        counts[0 if row["variant_kind"] == "clean" else 1] += 1
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
    for group, (clean_count, variant_count) in group_variants.items():
        require(clean_count == 1 and variant_count > 0,
                f"counterfactual group {group} needs exactly one clean reference and at least one fault variant")
    return by_id


def score(manifest: dict[str, Any], ledger: dict[str, Any],
          prediction_doc: dict[str, Any], bundle_root: Path) -> dict[str, Any]:
    candidate_by_id = validate_candidate(manifest, bundle_root)
    truth_by_id = validate_ledger(ledger)
    exact_keys(prediction_doc, {"schema_version", "benchmark_id", "predictions"},
               "predictions")
    require(is_int(prediction_doc["schema_version"])
            and prediction_doc["schema_version"] == 2,
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
            "detected_fault_families", "pose", "uncertainty_region",
            "orientation_method", "translation_method",
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
        if row["disposition"] == "qualified":
            require(row["pose"] is not None and row["uncertainty_region"] is not None
                    and is_string(row["orientation_method"])
                    and is_string(row["translation_method"]),
                    f"qualified prediction {eid} requires pose, region and component method IDs")
            pose = row["pose"]
            region = row["uncertainty_region"]
            exact_keys(pose, {
                "coordinate_frame", "transform_direction", "translation_m", "orientation_xyzw",
            }, f"pose {eid}")
            exact_keys(region, {
                "coordinate_frame", "transform_direction", "translation_lower_m",
                "translation_upper_m", "orientation_center_xyzw", "orientation_half_angle_rad",
            }, f"pose region {eid}")
            truth = truth_by_id.get(eid)
            require(truth is not None, f"prediction {eid} has no evaluator truth")
            truth_pose = truth["ground_truth_pose"]
            for obj, where in ((pose, "pose"), (region, "region")):
                require(obj["coordinate_frame"] == truth_pose["coordinate_frame"]
                        and obj["transform_direction"] == truth_pose["transform_direction"],
                        f"{where} coordinate frame / transform direction mismatch for {eid}")
            translation = validate_vec3(pose["translation_m"], f"pose translation {eid}")
            validate_quaternion(pose["orientation_xyzw"], f"pose quaternion {eid}")
            lower = validate_vec3(region["translation_lower_m"], f"region lower {eid}")
            upper = validate_vec3(region["translation_upper_m"], f"region upper {eid}")
            require(all(a <= b for a, b in zip(lower, upper, strict=True)),
                    f"region lower bound exceeds upper bound for {eid}")
            validate_quaternion(region["orientation_center_xyzw"], f"region quaternion {eid}")
            half_angle = region["orientation_half_angle_rad"]
            require(is_number(half_angle) and 0.0 <= float(half_angle) <= 3.141592653589793,
                    f"region orientation half-angle invalid for {eid}")
        else:
            require(row["pose"] is None and row["uncertainty_region"] is None
                    and row["orientation_method"] is None
                    and row["translation_method"] is None,
                    f"non-qualified prediction {eid} must not emit a pose")
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
    pose_metrics: list[dict[str, Any]] = []
    generalization_counts = {
        stratum: [0, 0, 0, 0, 0, 0, 0] for stratum in GENERALIZATION
    }

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
        if actual == "qualified":
            import math
            gt = truth["ground_truth_pose"]
            pred = prediction["pose"]
            region = prediction["uncertainty_region"]
            gt_translation = validate_vec3(gt["translation_m"], f"truth translation {eid}")
            pred_translation = validate_vec3(pred["translation_m"], f"predicted translation {eid}")
            gt_quaternion = validate_quaternion(gt["orientation_xyzw"], f"truth quaternion {eid}")
            pred_quaternion = validate_quaternion(pred["orientation_xyzw"], f"predicted quaternion {eid}")
            translation_error = math.sqrt(sum(
                (a - b) ** 2 for a, b in zip(gt_translation, pred_translation, strict=True)
            ))
            truth_range = math.sqrt(sum(x * x for x in gt_translation))
            relative_error = translation_error / truth_range if truth_range > 1e-9 else None
            rotation_error = quaternion_distance(pred_quaternion, gt_quaternion)
            lower = validate_vec3(region["translation_lower_m"], f"region lower {eid}")
            upper = validate_vec3(region["translation_upper_m"], f"region upper {eid}")
            translation_inside = all(
                low <= value <= high
                for value, low, high in zip(gt_translation, lower, upper, strict=True)
            )
            region_orientation = validate_quaternion(
                region["orientation_center_xyzw"], f"region quaternion {eid}"
            )
            orientation_inside = quaternion_distance(
                region_orientation, gt_quaternion
            ) <= float(region["orientation_half_angle_rad"]) + 1e-12
            pose_metrics.append({
                "episode_id": eid,
                "translation_error_m": translation_error,
                "relative_translation_error": relative_error,
                "rotation_error_rad": rotation_error,
                "rotation_error_deg": math.degrees(rotation_error),
                "uncertainty_region_covered": translation_inside and orientation_inside,
            })
        stratum_counts = generalization_counts[truth["generalization"]]
        stratum_counts[0] += 1
        if expected == "qualified":
            clean_episodes += 1
            stratum_counts[4] += 1
            if actual == "qualified":
                clean_qualified += 1
                stratum_counts[5] += 1
            else:
                false_abstentions += 1
                stratum_counts[6] += 1
        else:
            fault_episodes += 1
            critical_fault_episodes += truth["critical"]
            stratum_counts[1] += 1
            stratum_counts[2] += truth["critical"]
            if actual == "qualified":
                unsafe_accepts += 1
                critical_unsafe_accepts += truth["critical"]
                stratum_counts[3] += 1
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
    generalization = [
        {
            "stratum": stratum,
            "episodes": generalization_counts[stratum][0],
            "fault_episodes": generalization_counts[stratum][1],
            "critical_fault_episodes": generalization_counts[stratum][2],
            "unsafe_accepts": generalization_counts[stratum][3],
            "clean_episodes": generalization_counts[stratum][4],
            "clean_qualified": generalization_counts[stratum][5],
            "false_abstentions": generalization_counts[stratum][6],
        }
        for stratum in GENERALIZATION_ORDER
        if generalization_counts[stratum][0] > 0
    ]
    pose_estimates = len(pose_metrics)
    region_covered = sum(row["uncertainty_region_covered"] for row in pose_metrics)
    pose_summary = None
    if pose_metrics:
        pose_summary = {
            "estimates": pose_estimates,
            "region_coverage_denominator": pose_estimates,
            "region_covered": region_covered,
            "silent_misses": pose_estimates - region_covered,
            "median_translation_error_m": quantile([row["translation_error_m"] for row in pose_metrics], 0.5),
            "p95_translation_error_m": quantile([row["translation_error_m"] for row in pose_metrics], 0.95),
            "median_relative_translation_error": quantile(
                [row["relative_translation_error"] for row in pose_metrics if row["relative_translation_error"] is not None], 0.5
            ),
            "p95_relative_translation_error": quantile(
                [row["relative_translation_error"] for row in pose_metrics if row["relative_translation_error"] is not None], 0.95
            ),
            "median_rotation_error_rad": quantile([row["rotation_error_rad"] for row in pose_metrics], 0.5),
            "p95_rotation_error_rad": quantile([row["rotation_error_rad"] for row in pose_metrics], 0.95),
            "median_rotation_error_deg": quantile([row["rotation_error_deg"] for row in pose_metrics], 0.5),
            "p95_rotation_error_deg": quantile([row["rotation_error_deg"] for row in pose_metrics], 0.95),
        }

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
        "generalization": generalization,
        "pose_estimates": pose_estimates,
        "pose_region_covered": region_covered,
        "pose_silent_misses": pose_estimates - region_covered,
        "pose_metrics": pose_metrics,
        "pose_summary": pose_summary,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--ledger", type=Path, required=True)
    parser.add_argument("--predictions", type=Path, required=True)
    parser.add_argument("--bundle-root", type=Path, required=True)
    parser.add_argument("--expect-unsafe", type=int, required=True)
    args = parser.parse_args()
    try:
        result = score(read_json(args.manifest), read_json(args.ledger),
                       read_json(args.predictions), args.bundle_root)
        print(json.dumps(result, indent=2))
        require(result["unsafe_accepts"] == args.expect_unsafe,
                f"unsafe_accepts expected {args.expect_unsafe}, got {result['unsafe_accepts']}")
        return 0
    except (OSError, json.JSONDecodeError, ValueError, TypeError, KeyError) as error:
        print(f"cfi-space-challenger: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
