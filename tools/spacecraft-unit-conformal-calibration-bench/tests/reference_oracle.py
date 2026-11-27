#!/usr/bin/env python3
"""Independent oracle for unit-level spacecraft split-conformal calibration."""

from __future__ import annotations

import csv
import hashlib
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

SCHEMA = "luminous.spacecraft-unit-conformal-calibration.receipt.v0.1"
LEVELS_BPS = (5000, 8000, 9000, 9500, 9900)
MIN_HELDOUT_UNITS = 30
WILSON_Z_95 = 1.959963984540054
Q32_INFINITY_SENTINEL = 2**64 - 1
GROUP_FIELD = {"TARGET": "target_id", "SESSION": "session_id", "SEQUENCE": "sequence_id"}
STATUSES = {"SCORED", "ABSTAINED", "MISSING", "INVALID_INPUT"}


def validate_rows(rows: list[dict[str, str]]) -> str:
    if not rows:
        raise ValueError("manifest has no observations")
    unit_kind = rows[0]["unit_kind"]
    if unit_kind not in GROUP_FIELD:
        raise ValueError(f"unsupported unit_kind: {unit_kind}")
    seen_samples: set[str] = set()
    unit_partitions: dict[tuple[str, str], str] = {}

    for index, row in enumerate(rows, start=2):
        for field in (
            "sample_id", "target_id", "session_id", "sequence_id", "unit_id",
            "domain_id", "condition_id", "crop_stratum", "modality", "ood_group",
            "calibration_revision",
        ):
            if not row[field].strip() or row[field] == "NONE":
                raise ValueError(f"row {index}: {field} is empty or reserved")
        if row["unit_kind"] != unit_kind:
            raise ValueError(f"row {index}: unit_kind changes within manifest")
        if row["unit_kind"] not in GROUP_FIELD:
            raise ValueError(f"row {index}: unsupported unit_kind")
        if row["unit_id"] != row[GROUP_FIELD[unit_kind]]:
            raise ValueError(f"row {index}: unit_id does not match declared identity")
        if row["split"] not in ("CALIBRATION", "TEST"):
            raise ValueError(f"row {index}: unsupported split {row['split']}")
        budget = int(row["unit_frame_budget"])
        if budget <= 0:
            raise ValueError(f"row {index}: unit_frame_budget must be positive")
        if row["status"] not in STATUSES:
            raise ValueError(f"row {index}: unsupported observation status")

        sample = row["sample_id"]
        if sample in seen_samples:
            raise ValueError(f"duplicate sample_id: {sample}")
        seen_samples.add(sample)

        identity = (unit_kind, row["unit_id"])
        previous = unit_partitions.setdefault(identity, row["split"])
        if previous != row["split"]:
            raise ValueError(f"independent unit {row['unit_id']} crosses splits")

        raw_score = row["score_q32_32"]
        score = None if raw_score == "NONE" else int(raw_score)
        if row["status"] == "SCORED":
            if score is None or score == Q32_INFINITY_SENTINEL or score < 0:
                raise ValueError(f"row {index}: SCORED requires a finite Q32.32 score")
        elif score is not None:
            raise ValueError(f"row {index}: unscored rows must carry NONE score")

    return unit_kind


def scope_tuple(row: dict[str, str]) -> tuple[str, ...]:
    return (
        row["domain_id"], row["condition_id"], row["crop_stratum"],
        row["modality"], row["ood_group"], row["calibration_revision"],
        row["unit_frame_budget"],
    )


def build_units(rows: list[dict[str, str]], split: str) -> dict[tuple[str, ...], dict[str, dict[str, object]]]:
    scopes: dict[tuple[str, ...], dict[str, dict[str, object]]] = defaultdict(dict)
    for row in rows:
        if row["split"] != split:
            continue
        scope = scope_tuple(row)
        unit_id = row["unit_id"]
        unit = scopes[scope].setdefault(
            unit_id,
            {
                "partition": split,
                "scores": [],
                "scored": 0,
                "abstained": 0,
                "missing": 0,
                "invalid": 0,
            },
        )
        raw_score = row["score_q32_32"]
        score = None if raw_score == "NONE" else int(raw_score)
        unit["scores"].append(score)
        if row["status"] == "SCORED":
            unit["scored"] += 1
        elif row["status"] == "ABSTAINED":
            unit["abstained"] += 1
        elif row["status"] == "MISSING":
            unit["missing"] += 1
        else:
            unit["invalid"] += 1

    for scope, units in scopes.items():
        budget = int(scope[-1])
        for unit_id, unit in units.items():
            if len(unit["scores"]) != budget:
                raise ValueError(
                    f"unit {unit_id} frame count {len(unit['scores'])} differs from budget {budget}"
                )
    return scopes


def unit_max(unit: dict[str, object]) -> int | None:
    scores = unit["scores"]
    if any(score is None for score in scores):
        return None
    return max(scores)


def conformal_quantile(scores: list[int | None], level_bps: int) -> tuple[int, int | None]:
    n = len(scores)
    rank = ((n + 1) * level_bps + 9999) // 10000
    if rank > n:
        return rank, None
    ordered = sorted(scores, key=lambda value: (value is None, value if value is not None else 0))
    return rank, ordered[rank - 1]


def ppm_ratio(numerator: int, denominator: int) -> int | None:
    if denominator == 0:
        return None
    return numerator * 1_000_000 // denominator


def wilson_interval_95(successes: int, trials: int) -> dict[str, int] | None:
    if trials == 0 or successes > trials:
        return None
    n = float(trials)
    p = float(successes) / n
    z2 = WILSON_Z_95 * WILSON_Z_95
    denominator = 1.0 + z2 / n
    center = (p + z2 / (2.0 * n)) / denominator
    variance = p * (1.0 - p) / n + z2 / (4.0 * n * n)
    half = WILSON_Z_95 * math.sqrt(variance) / denominator
    lower = min(1.0, max(0.0, center - half))
    upper = min(1.0, max(0.0, center + half))
    return {
        "lower_ppm": math.floor(lower * 1_000_000.0),
        "upper_ppm": math.ceil(upper * 1_000_000.0),
    }


def evaluate_threshold(
    units: dict[str, dict[str, object]], threshold: int | None
) -> dict[str, int] | None:
    if threshold is None:
        return None
    covered_frames = 0
    complete_units = 0
    silent_miss_units = 0
    units_with_unscored = 0
    for unit in units.values():
        scores = unit["scores"]
        if any(score is None for score in scores):
            units_with_unscored += 1
        if any(score is not None and score > threshold for score in scores):
            silent_miss_units += 1
        complete = True
        for score in scores:
            if score is not None and score <= threshold:
                covered_frames += 1
            else:
                complete = False
        if complete:
            complete_units += 1
    return {
        "covered_frames": covered_frames,
        "complete_units": complete_units,
        "silent_miss_units": silent_miss_units,
        "units_with_unscored": units_with_unscored,
    }


def count_status(units: dict[str, dict[str, object]]) -> tuple[int, int, int, int, int]:
    frames = scored = abstained = missing = invalid = 0
    for unit in units.values():
        frames += len(unit["scores"])
        scored += unit["scored"]
        abstained += unit["abstained"]
        missing += unit["missing"]
        invalid += unit["invalid"]
    return frames, scored, abstained, missing, invalid


def level_result(
    nominal: int,
    calibration_frames: list[int | None],
    calibration_units: list[int | None],
    test_units: dict[str, dict[str, object]],
) -> dict[str, object]:
    frame_rank, frame_threshold = conformal_quantile(calibration_frames, nominal)
    unit_rank, unit_threshold = conformal_quantile(calibration_units, nominal)
    test_frames, test_scored, test_abstained, test_missing, test_invalid = count_status(test_units)
    test_unit_count = len(test_units)
    frame_metrics = evaluate_threshold(test_units, frame_threshold)
    unit_metrics = evaluate_threshold(test_units, unit_threshold)

    def metric(metrics: dict[str, int] | None, name: str) -> int | None:
        return None if metrics is None else metrics[name]

    frame_covered = metric(frame_metrics, "covered_frames")
    unit_covered = metric(unit_metrics, "covered_frames")
    frame_complete = metric(frame_metrics, "complete_units")
    unit_complete = metric(unit_metrics, "complete_units")
    frame_silent = metric(frame_metrics, "silent_miss_units")
    unit_silent = metric(unit_metrics, "silent_miss_units")
    frame_ci = wilson_interval_95(frame_complete, test_unit_count) if frame_complete is not None else None
    unit_ci = wilson_interval_95(unit_complete, test_unit_count) if unit_complete is not None else None
    frame_silent_ci = wilson_interval_95(frame_silent, test_unit_count) if frame_silent is not None else None
    unit_silent_ci = wilson_interval_95(unit_silent, test_unit_count) if unit_silent is not None else None

    nominal_ppm = nominal * 100
    if unit_threshold is None:
        reason = "NO_FINITE_UNIT_CONFORMAL_THRESHOLD"
    elif test_unit_count < MIN_HELDOUT_UNITS:
        reason = "INSUFFICIENT_INDEPENDENT_TEST_UNITS"
    elif unit_ci is None or unit_ci["lower_ppm"] < nominal_ppm:
        reason = "HELDOUT_UNIT_COVERAGE_LOWER_BOUND_BELOW_NOMINAL"
    else:
        reason = "UNIT_LEVEL_HOLDOUT_TARGET_MET_FOR_DECLARED_SCOPE"

    return {
        "nominal_level_bps": nominal,
        "calibration_frame_count": len(calibration_frames),
        "calibration_unit_count": len(calibration_units),
        "framewise_order_statistic_rank": frame_rank,
        "unitwise_order_statistic_rank": unit_rank,
        "framewise_threshold_q32_32": frame_threshold,
        "unitwise_threshold_q32_32": unit_threshold,
        "framewise_finite": frame_threshold is not None,
        "unitwise_finite": unit_threshold is not None,
        "test_frame_count": test_frames,
        "test_unit_count": test_unit_count,
        "test_scored_frame_count": test_scored,
        "test_abstained_frame_count": test_abstained,
        "test_missing_frame_count": test_missing,
        "test_invalid_frame_count": test_invalid,
        "framewise_covered_frame_count": frame_covered,
        "framewise_conditional_coverage_ppm": ppm_ratio(frame_covered, test_scored) if frame_covered is not None else None,
        "framewise_operational_success_ppm": ppm_ratio(frame_covered, test_frames) if frame_covered is not None else None,
        "framewise_complete_unit_success_count": frame_complete,
        "framewise_complete_unit_wilson_95": frame_ci,
        "framewise_silent_miss_unit_count": frame_silent,
        "framewise_silent_miss_wilson_95": frame_silent_ci,
        "unitwise_covered_frame_count": unit_covered,
        "unitwise_conditional_coverage_ppm": ppm_ratio(unit_covered, test_scored) if unit_covered is not None else None,
        "unitwise_operational_success_ppm": ppm_ratio(unit_covered, test_frames) if unit_covered is not None else None,
        "unitwise_complete_unit_success_count": unit_complete,
        "unitwise_complete_unit_wilson_95": unit_ci,
        "unitwise_silent_miss_unit_count": unit_silent,
        "unitwise_silent_miss_wilson_95": unit_silent_ci,
        "unitwise_units_with_any_unscored_frame": metric(unit_metrics, "units_with_unscored"),
        "promotion_eligible_for_declared_scope": reason == "UNIT_LEVEL_HOLDOUT_TARGET_MET_FOR_DECLARED_SCOPE",
        "promotion_reason": reason,
    }


def evaluate(rows: list[dict[str, str]], unit_kind: str) -> dict[str, object]:
    calibration = build_units(rows, "CALIBRATION")
    test = build_units(rows, "TEST")
    scopes = sorted(set(calibration) | set(test))
    result_scopes = []

    for scope in scopes:
        calibration_units = calibration.get(scope, {})
        test_units = test.get(scope, {})
        frame_scores = [
            score
            for unit_id in sorted(calibration_units)
            for score in calibration_units[unit_id]["scores"]
        ]
        maxima = [unit_max(calibration_units[unit_id]) for unit_id in sorted(calibration_units)]
        unscored_frames = sum(
            score is None
            for unit in calibration_units.values()
            for score in unit["scores"]
        )
        unscored_units = sum(
            any(score is None for score in unit["scores"])
            for unit in calibration_units.values()
        )
        levels = [
            level_result(level, frame_scores, maxima, test_units)
            for level in LEVELS_BPS
        ]
        result_scopes.append({
            "domain_id": scope[0],
            "condition_id": scope[1],
            "crop_stratum": scope[2],
            "modality": scope[3],
            "ood_group": scope[4],
            "calibration_revision": scope[5],
            "unit_frame_budget": int(scope[6]),
            "unit_kind": unit_kind,
            "calibration_unscored_frame_count": unscored_frames,
            "calibration_units_with_unscored_frames": unscored_units,
            "levels": levels,
        })

    return {
        "schema": SCHEMA,
        "unit_kind": unit_kind,
        "observation_count": len(rows),
        "scope_count": len(result_scopes),
        "results": result_scopes,
        "claim_ready": False,
        "dataset_source_bytes_authenticated": False,
        "calibration_exchangeability_proven": False,
        "physical_calibration_validated": False,
        "deterministic_safety_bound": False,
        "maneuver_authority_granted": False,
    }


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: reference_oracle.py <observations.tsv> <source-commit> <corpus-sha256>"
        )
    path = Path(sys.argv[1])
    raw = path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != sys.argv[3]:
        raise SystemExit("manifest SHA-256 does not match input bytes")
    with path.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream, delimiter="\t"))
    unit_kind = validate_rows(rows)
    receipt = evaluate(rows, unit_kind)
    receipt["source_commit"] = sys.argv[2]
    receipt["corpus_sha256"] = digest
    receipt = {
        "schema": receipt.pop("schema"),
        "source_commit": receipt.pop("source_commit"),
        "corpus_sha256": receipt.pop("corpus_sha256"),
        **receipt,
    }
    print(json.dumps(receipt, sort_keys=True, separators=(",", ":"), allow_nan=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
