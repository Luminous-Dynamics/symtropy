#!/usr/bin/env python3
"""Minimum sufficient FPS evaluator over evidence-grounded AV task outcomes.

This is a scorecard adapter, not a video sampler or candidate runtime. It
validates a fixed, nested FPS sweep and computes the minimum *tested* rate
whose grounded-success fraction meets a preregistered threshold at that and
every higher tested rate.

Failures remain in the denominator. A run is successful only when it completed
and its linked evidence-certificate scorecard says grounded_success=true.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path
from statistics import mean
from typing import Any

BENCHMARK_ID = "av-minfps-curve-v0"
SCHEMA_VERSION = 1
ENERGY_PROVENANCE = {"MEASURED", "ESTIMATED", "SYNTHETIC", "NOT_MEASURED"}


class MinFpsEvaluationError(ValueError):
    """Invalid, incomplete or incomparable FPS sweep."""


def _finite_number(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
    )


def _nonempty(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise MinFpsEvaluationError(f"{field} must be a non-empty string")
    return value


def _digest(value: Any, field: str) -> str:
    if not isinstance(value, str) or len(value) != 64:
        raise MinFpsEvaluationError(f"{field} must be a 64-character SHA-256 digest")
    try:
        int(value, 16)
    except ValueError as exc:
        raise MinFpsEvaluationError(f"{field} must be hexadecimal") from exc
    return value.lower()


def _nonnegative_int(value: Any, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise MinFpsEvaluationError(f"{field} must be a non-negative integer")
    return value


def _optional_nonnegative(value: Any, field: str) -> int | None:
    if value is None:
        return None
    return _nonnegative_int(value, field)


def _validate_rate_grid(value: Any) -> list[float]:
    if not isinstance(value, list) or len(value) < 2:
        raise MinFpsEvaluationError("fps_grid must contain at least two rates")
    rates: list[float] = []
    for index, rate in enumerate(value):
        if not _finite_number(rate) or float(rate) <= 0.0:
            raise MinFpsEvaluationError(f"fps_grid[{index}] must be finite and positive")
        rates.append(float(rate))
    if rates != sorted(rates) or len(set(rates)) != len(rates):
        raise MinFpsEvaluationError("fps_grid must be strictly increasing and unique")
    return rates


def _validate_resource(row: Any, context: str) -> dict[str, Any]:
    required = {
        "frames_delivered",
        "video_bytes",
        "audio_samples",
        "audio_bytes",
        "input_tokens",
        "output_tokens",
        "model_calls",
        "wall_time_ms",
        "peak_ram_mb",
        "peak_vram_mb",
        "energy_millijoules",
        "energy_provenance",
        "energy_method",
        "energy_evidence_digest_sha256",
    }
    if not isinstance(row, dict) or set(row) != required:
        raise MinFpsEvaluationError(f"{context}: resource telemetry has missing or unknown fields")

    result: dict[str, Any] = {}
    for key in (
        "frames_delivered",
        "video_bytes",
        "audio_samples",
        "audio_bytes",
        "input_tokens",
        "output_tokens",
        "model_calls",
        "wall_time_ms",
        "peak_ram_mb",
        "peak_vram_mb",
    ):
        result[key] = _optional_nonnegative(row[key], f"{context}.{key}")

    energy = row["energy_millijoules"]
    provenance = row["energy_provenance"]
    if not isinstance(provenance, str) or provenance not in ENERGY_PROVENANCE:
        raise MinFpsEvaluationError(f"{context}: invalid energy_provenance")
    method = row["energy_method"]
    evidence_digest = row["energy_evidence_digest_sha256"]
    if energy is not None and (not _finite_number(energy) or float(energy) < 0.0):
        raise MinFpsEvaluationError(f"{context}: energy_millijoules must be non-negative or null")
    if provenance == "MEASURED":
        if energy is None:
            raise MinFpsEvaluationError(f"{context}: MEASURED energy requires a value")
        if not isinstance(method, str) or not method:
            raise MinFpsEvaluationError(f"{context}: MEASURED energy requires energy_method")
        result["energy_evidence_digest_sha256"] = _digest(
            evidence_digest, f"{context}.energy_evidence_digest_sha256"
        )
    else:
        if energy is not None and provenance == "NOT_MEASURED":
            raise MinFpsEvaluationError(
                f"{context}: NOT_MEASURED energy must be null, not an assumed value"
            )
        if provenance == "ESTIMATED" and not isinstance(method, str):
            raise MinFpsEvaluationError(f"{context}: ESTIMATED energy requires a method label")
        if evidence_digest is not None:
            raise MinFpsEvaluationError(
                f"{context}: evidence digest is reserved for independently evidenced measured energy"
            )
        result["energy_evidence_digest_sha256"] = None

    result.update({
        "energy_millijoules": float(energy) if energy is not None else None,
        "energy_provenance": provenance,
        "energy_method": method,
    })
    return result


def _validate_frame_manifest(row: Any, context: str) -> list[dict[str, Any]]:
    if not isinstance(row, list):
        raise MinFpsEvaluationError(f"{context}: frame_manifest must be an array")
    seen: set[str] = set()
    result: list[dict[str, Any]] = []
    previous_timestamp: float | None = None
    for index, frame in enumerate(row):
        if not isinstance(frame, dict) or set(frame) != {"frame_id", "timestamp_ms"}:
            raise MinFpsEvaluationError(f"{context}: frame {index} has missing or unknown fields")
        frame_id = _nonempty(frame["frame_id"], f"{context}.frame_id")
        timestamp = frame["timestamp_ms"]
        if frame_id in seen:
            raise MinFpsEvaluationError(f"{context}: duplicate frame_id {frame_id}")
        if not _finite_number(timestamp) or float(timestamp) < 0.0:
            raise MinFpsEvaluationError(f"{context}: frame timestamp must be finite and non-negative")
        timestamp = float(timestamp)
        if previous_timestamp is not None and timestamp <= previous_timestamp:
            raise MinFpsEvaluationError(f"{context}: frame timestamps must be strictly increasing")
        seen.add(frame_id)
        result.append({"frame_id": frame_id, "timestamp_ms": timestamp})
        previous_timestamp = timestamp
    return result


def _validate_rate_run(
    row: Any,
    expected_rate: float,
    context: str,
) -> dict[str, Any]:
    required = {
        "fps",
        "run_status",
        "frame_manifest",
        "frame_manifest_sha256",
        "grounded_success",
        "answer_correct",
        "evidence_sufficient",
        "decision_on_time",
        "certificate_scorecard_sha256",
        "resource",
    }
    if not isinstance(row, dict) or set(row) != required:
        raise MinFpsEvaluationError(f"{context}: rate record has missing or unknown fields")
    if not _finite_number(row["fps"]) or float(row["fps"]) != expected_rate:
        raise MinFpsEvaluationError(f"{context}: FPS rate does not match frozen grid")
    status = row["run_status"]
    if not isinstance(status, str) or status not in {"COMPLETED", "TIMEOUT", "FAILED", "MALFORMED"}:
        raise MinFpsEvaluationError(f"{context}: invalid run_status")
    frames = _validate_frame_manifest(row["frame_manifest"], context)
    manifest_digest = _digest(row["frame_manifest_sha256"], f"{context}.frame_manifest_sha256")
    canonical_frames = json.dumps(
        frames, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    expected_digest = hashlib.sha256(canonical_frames).hexdigest()
    if manifest_digest != expected_digest:
        raise MinFpsEvaluationError(f"{context}: frame_manifest digest mismatch")
    resource = _validate_resource(row["resource"], f"{context}.resource")
    if resource["frames_delivered"] is None or resource["frames_delivered"] != len(frames):
        raise MinFpsEvaluationError(
            f"{context}: frames_delivered must equal the frame manifest count"
        )

    if status == "COMPLETED":
        for field in ("grounded_success", "answer_correct", "evidence_sufficient", "decision_on_time"):
            if not isinstance(row[field], bool):
                raise MinFpsEvaluationError(f"{context}: {field} must be boolean for completed runs")
        certificate = _digest(
            row["certificate_scorecard_sha256"],
            f"{context}.certificate_scorecard_sha256",
        )
        # Grounded success is not a candidate assertion: it is the linked output
        # of the independent #6856 evaluator, additionally constrained here.
        grounded = (
            row["grounded_success"]
            and row["answer_correct"]
            and row["evidence_sufficient"]
            and row["decision_on_time"]
        )
    else:
        if any(row[field] is not None for field in (
            "grounded_success",
            "answer_correct",
            "evidence_sufficient",
            "decision_on_time",
            "certificate_scorecard_sha256",
        )):
            raise MinFpsEvaluationError(
                f"{context}: failed runs must not contain candidate scorecard fields"
            )
        certificate = None
        grounded = False

    return {
        "fps": expected_rate,
        "run_status": status,
        "frame_manifest": frames,
        "frame_manifest_sha256": manifest_digest,
        "grounded_success": grounded,
        "answer_correct": row["answer_correct"],
        "evidence_sufficient": row["evidence_sufficient"],
        "decision_on_time": row["decision_on_time"],
        "certificate_scorecard_sha256": certificate,
        "resource": resource,
    }


def _rate_summary(
    rate: float,
    runs: list[dict[str, Any]],
    min_success_fraction: float,
) -> dict[str, Any]:
    count = len(runs)
    successes = sum(bool(row["grounded_success"]) for row in runs)
    fraction = successes / count if count else 0.0
    fields = (
        "frames_delivered",
        "video_bytes",
        "audio_samples",
        "audio_bytes",
        "input_tokens",
        "output_tokens",
        "model_calls",
        "wall_time_ms",
        "peak_ram_mb",
        "peak_vram_mb",
    )
    resource_means = {}
    resource_observed_counts = {}
    for field in fields:
        values = [
            row["resource"][field]
            for row in runs
            if row["resource"][field] is not None
        ]
        resource_means[field] = mean(values) if values else None
        resource_observed_counts[field] = len(values)

    measured_energy = [
        row["resource"]["energy_millijoules"]
        for row in runs
        if row["resource"]["energy_provenance"] == "MEASURED"
    ]
    energy_provenance_counts = {
        provenance: sum(
            row["resource"]["energy_provenance"] == provenance for row in runs
        )
        for provenance in sorted(ENERGY_PROVENANCE)
    }
    return {
        "fps": rate,
        "seeds": count,
        "grounded_successes": successes,
        "grounded_success_fraction": fraction,
        "meets_success_threshold": fraction >= min_success_fraction,
        "mean_resource_metrics": resource_means,
        "resource_observed_counts": resource_observed_counts,
        "measured_energy_millijoules_mean": (
            mean(measured_energy) if measured_energy else None
        ),
        "measured_energy_sample_count": len(measured_energy),
        "energy_provenance_counts": energy_provenance_counts,
    }


def _minimum_fps(
    rates: list[float],
    summaries: list[dict[str, Any]],
) -> dict[str, Any]:
    flags = [bool(row["meets_success_threshold"]) for row in summaries]
    if all(flags):
        return {
            "status": "LEFT_CENSORED",
            "min_fps": rates[0],
            "explanation": "lowest tested rate already meets the grounded-success threshold",
        }
    if not any(flags):
        return {
            "status": "RIGHT_CENSORED",
            "min_fps": None,
            "explanation": "highest tested rate fails the grounded-success threshold",
        }
    for index, passes in enumerate(flags):
        if passes and all(flags[index:]):
            if index == 0:
                return {
                    "status": "LEFT_CENSORED",
                    "min_fps": rates[0],
                    "explanation": "lowest tested rate meets the grounded-success threshold",
                }
            return {
                "status": "MIN_FPS_FOUND",
                "min_fps": rates[index],
                "explanation": "this is the lowest tested rate with a passing monotone suffix",
            }
    return {
        "status": "NO_STABLE_RATE",
        "min_fps": None,
        "explanation": "grounded success is non-monotone and no tested rate has a passing higher-rate suffix",
    }


def evaluate_document(document: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(document, dict):
        raise MinFpsEvaluationError("input root must be an object")
    if document.get("benchmark_id") != BENCHMARK_ID:
        raise MinFpsEvaluationError("benchmark_id mismatch")
    version = document.get("schema_version")
    if isinstance(version, bool) or not isinstance(version, int) or version != SCHEMA_VERSION:
        raise MinFpsEvaluationError("schema_version must be integer 1")

    protocol_id = _nonempty(document.get("protocol_id"), "protocol_id")
    episode_id = _nonempty(document.get("episode_id"), "episode_id")
    source_cluster_id = _nonempty(document.get("source_cluster_id"), "source_cluster_id")
    policy_id = _nonempty(document.get("policy_id"), "policy_id")
    model_digest = _digest(document.get("model_digest_sha256"), "model_digest_sha256")
    runtime_digest = _digest(document.get("runtime_digest_sha256"), "runtime_digest_sha256")
    sampler_id = _nonempty(document.get("sampler_id"), "sampler_id")
    sampler_digest = _digest(document.get("sampler_digest_sha256"), "sampler_digest_sha256")

    rates = _validate_rate_grid(document.get("fps_grid"))
    media_duration_ms = document.get("media_duration_ms")
    if not _finite_number(media_duration_ms) or float(media_duration_ms) <= 0.0:
        raise MinFpsEvaluationError("media_duration_ms must be finite and positive")
    media_duration_ms = float(media_duration_ms)
    sampling_tolerance_ms = document.get("sampling_tolerance_ms")
    if not _finite_number(sampling_tolerance_ms) or float(sampling_tolerance_ms) <= 0.0:
        raise MinFpsEvaluationError("sampling_tolerance_ms must be finite and positive")
    sampling_tolerance_ms = float(sampling_tolerance_ms)
    min_fraction = document.get("minimum_success_fraction", 1.0)
    min_seed_count = document.get("minimum_seed_count", 1)
    if not _finite_number(min_fraction) or not 0.0 < float(min_fraction) <= 1.0:
        raise MinFpsEvaluationError("minimum_success_fraction must be in (0,1]")
    if isinstance(min_seed_count, bool) or not isinstance(min_seed_count, int) or min_seed_count < 1:
        raise MinFpsEvaluationError("minimum_seed_count must be an integer >= 1")
    min_fraction = float(min_fraction)

    seeds = document.get("seeds")
    if not isinstance(seeds, list) or not seeds:
        raise MinFpsEvaluationError("seeds must be a non-empty array")
    seed_ids: list[str] = []
    parsed_seed_rows: dict[str, list[dict[str, Any]]] = {}
    for seed_index, seed in enumerate(seeds):
        if not isinstance(seed, dict) or set(seed) != {"seed_id", "runs"}:
            raise MinFpsEvaluationError(f"seed {seed_index}: expected only seed_id and runs")
        seed_id = _nonempty(seed["seed_id"], "seed_id")
        if seed_id in parsed_seed_rows:
            raise MinFpsEvaluationError(f"duplicate seed_id: {seed_id}")
        seed_ids.append(seed_id)
        rows = seed["runs"]
        if not isinstance(rows, list) or len(rows) != len(rates):
            raise MinFpsEvaluationError(
                f"{seed_id}: every tested FPS must have exactly one attempted run"
            )
        parsed = [
            _validate_rate_run(row, rate, f"{seed_id}@{rate:g}fps")
            for row, rate in zip(rows, rates)
        ]
        parsed_seed_rows[seed_id] = parsed

        # Validate the delivered temporal density against the declared fixed
        # FPS schedule, then check nesting and timestamp identity.
        for index, run in enumerate(parsed):
            expected_frames = max(
                1,
                math.ceil((media_duration_ms / 1000.0) * rates[index] - 1e-12),
            )
            frame_manifest = run["frame_manifest"]
            if len(frame_manifest) > expected_frames:
                raise MinFpsEvaluationError(
                    f"{seed_id}@{rates[index]:g}fps: delivered frame count exceeds the nominal schedule"
                )
            expected_period = 1000.0 / rates[index]
            for frame in frame_manifest:
                schedule_index = round(frame["timestamp_ms"] / expected_period)
                scheduled_timestamp = schedule_index * expected_period
                if abs(frame["timestamp_ms"] - scheduled_timestamp) > sampling_tolerance_ms:
                    raise MinFpsEvaluationError(
                        f"{seed_id}@{rates[index]:g}fps: frame timestamp is outside the fixed-rate schedule"
                    )
                if frame["timestamp_ms"] > media_duration_ms:
                    raise MinFpsEvaluationError(
                        f"{seed_id}@{rates[index]:g}fps: frame timestamp exceeds media duration"
                    )
            if run["run_status"] == "COMPLETED":
                if len(frame_manifest) != expected_frames:
                    raise MinFpsEvaluationError(
                        f"{seed_id}@{rates[index]:g}fps: completed run omitted scheduled frames"
                    )
                if frame_manifest[0]["timestamp_ms"] > sampling_tolerance_ms:
                    raise MinFpsEvaluationError(
                        f"{seed_id}@{rates[index]:g}fps: fixed-rate sampling must begin at the declared zero-time phase"
                    )
                for earlier, later in zip(frame_manifest, frame_manifest[1:]):
                    delta = later["timestamp_ms"] - earlier["timestamp_ms"]
                    if abs(delta - expected_period) > sampling_tolerance_ms:
                        raise MinFpsEvaluationError(
                            f"{seed_id}@{rates[index]:g}fps: frame spacing does not match the declared fixed rate"
                        )

        # Compare only completed sampling manifests: failed attempts may have
        # partial delivery, which remains in the outcome denominator.
        completed_indices = [
            index for index, run in enumerate(parsed)
            if run["run_status"] == "COMPLETED"
        ]
        for left_position, left_index in enumerate(completed_indices):
            for right_index in completed_indices[left_position + 1:]:
                lower = {
                    frame["frame_id"]: frame["timestamp_ms"]
                    for frame in parsed[left_index]["frame_manifest"]
                }
                higher = {
                    frame["frame_id"]: frame["timestamp_ms"]
                    for frame in parsed[right_index]["frame_manifest"]
                }
                if not set(lower).issubset(higher):
                    raise MinFpsEvaluationError(
                        f"{seed_id}: frame set at {rates[left_index]:g} FPS is not nested in {rates[right_index]:g} FPS"
                    )
                if any(lower[frame_id] != higher[frame_id] for frame_id in lower):
                    raise MinFpsEvaluationError(
                        f"{seed_id}: a shared frame ID has different timestamps across FPS conditions"
                    )

    if len(seed_ids) < min_seed_count:
        raise MinFpsEvaluationError("seed count is below the preregistered minimum_seed_count")

    rate_summaries = []
    for rate_index, rate in enumerate(rates):
        runs = [parsed_seed_rows[seed_id][rate_index] for seed_id in seed_ids]
        rate_summaries.append(_rate_summary(rate, runs, min_fraction))

    min_result = _minimum_fps(rates, rate_summaries)
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": SCHEMA_VERSION,
        "protocol_id": protocol_id,
        "episode_id": episode_id,
        "source_cluster_id": source_cluster_id,
        "policy_id": policy_id,
        "model_digest_sha256": model_digest,
        "runtime_digest_sha256": runtime_digest,
        "sampler_id": sampler_id,
        "sampler_digest_sha256": sampler_digest,
        "fps_grid": rates,
        "media_duration_ms": media_duration_ms,
        "sampling_tolerance_ms": sampling_tolerance_ms,
        "seed_ids": seed_ids,
        "minimum_success_fraction": min_fraction,
        "minimum_seed_count": min_seed_count,
        "attempted_rate_runs": len(seed_ids) * len(rates),
        "failed_rate_runs": sum(
            row["run_status"] != "COMPLETED"
            for seed_runs in parsed_seed_rows.values()
            for row in seed_runs
        ),
        "rate_summaries": rate_summaries,
        "minimum_fps": min_result,
        "claim_boundary": (
            "Minimum tested rate is defined over the preregistered rate grid and "
            "grounded-success threshold. It is not interpolated to a continuous "
            "threshold and is not a universal FPS requirement."
        ),
    }


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise MinFpsEvaluationError(f"{path}: expected JSON object")
    return value


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        result = evaluate_document(read_json(args.input))
    except (OSError, json.JSONDecodeError, MinFpsEvaluationError) as exc:
        print(json.dumps({"status": "INVALID", "error": str(exc)}), file=sys.stderr)
        raise SystemExit(2) from exc
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")


if __name__ == "__main__":
    main()
