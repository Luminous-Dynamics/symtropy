#!/usr/bin/env python3
"""Line-oriented streaming driver for the bounded paired-policy evaluator.

Input is NDJSON. The first record is a start envelope. Each subsequent record
is one independently sampled source-cluster outcome. A terminal decision is
emitted immediately and the driver stops reading input, allowing a cooperating
producer to avoid scheduling further evaluations.

This is still a one-metric, two-policy, bounded-mean method. It does not
implement multi-arm rank sets or metric-family multiplicity.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from typing import Any, Callable, Iterable, TextIO

from anytime_valid_policy_eval import EvaluationError, anytime_interval, finite_number


def _nonempty_string(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise EvaluationError(f"{field} must be a non-empty string")
    return value


def _positive_int(value: Any, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 1:
        raise EvaluationError(f"{field} must be an integer >= 1")
    return value


def _read_ndjson(stream: TextIO) -> Iterable[dict[str, Any]]:
    for line_number, line in enumerate(stream, start=1):
        if not line.strip():
            continue
        try:
            item = json.loads(line)
        except json.JSONDecodeError as exc:
            raise EvaluationError(f"line {line_number}: invalid JSON: {exc.msg}") from exc
        if not isinstance(item, dict):
            raise EvaluationError(f"line {line_number}: every record must be a JSON object")
        yield item


def run_stream(
    records: Iterable[dict[str, Any]],
    emit: Callable[[dict[str, Any]], None],
) -> dict[str, Any]:
    """Consume records lazily and stop requesting input at the first terminal decision."""
    iterator = iter(records)
    try:
        start = next(iterator)
    except StopIteration as exc:
        raise EvaluationError("missing start record") from exc

    if not isinstance(start, dict):
        raise EvaluationError("start record must be an object")
    allowed_start = {
        "type", "schema_version", "sampling_unit", "alpha",
        "superiority_margin", "max_observations", "metric_id",
        "candidate_policy_id", "reference_policy_id",
    }
    if set(start) - allowed_start:
        raise EvaluationError("start record contains unknown fields")
    if start.get("type") != "start":
        raise EvaluationError("first record must have type=start")
    version = start.get("schema_version")
    if isinstance(version, bool) or not isinstance(version, int) or version != 1:
        raise EvaluationError("schema_version must be integer 1")
    if start.get("sampling_unit") != "independent_source_cluster":
        raise EvaluationError("sampling_unit must be independent_source_cluster")

    alpha = start.get("alpha", 0.05)
    margin = start.get("superiority_margin", 0.0)
    if not finite_number(alpha) or not 0.0 < alpha < 1.0:
        raise EvaluationError("alpha must be finite and strictly between 0 and 1")
    if not finite_number(margin) or not -1.0 <= margin <= 1.0:
        raise EvaluationError("superiority_margin must be finite and in [-1, 1]")
    max_observations = _positive_int(start.get("max_observations"), "max_observations")

    metric_id = _nonempty_string(start.get("metric_id", "unspecified"), "metric_id")
    candidate_id = _nonempty_string(
        start.get("candidate_policy_id", "candidate"), "candidate_policy_id"
    )
    reference_id = _nonempty_string(
        start.get("reference_policy_id", "reference"), "reference_policy_id"
    )
    start_commit = {
        "schema_version": 1,
        "sampling_unit": "independent_source_cluster",
        "alpha": float(alpha),
        "superiority_margin": float(margin),
        "max_observations": max_observations,
        "metric_id": metric_id,
        "candidate_policy_id": candidate_id,
        "reference_policy_id": reference_id,
    }
    start_bytes = json.dumps(
        start_commit, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    chain = hashlib.sha256(b"av-anytime-valid-stream-v1" + start_bytes).digest()

    seen_clusters: set[str] = set()
    total = 0.0
    n = 0
    latest: dict[str, Any] | None = None

    for record in iterator:
        if not isinstance(record, dict):
            raise EvaluationError("each stream record must be an object")
        kind = record.get("type")
        if kind == "end":
            if set(record) != {"type"}:
                raise EvaluationError("end record may contain only type")
            result = {
                "type": "decision",
                "status": "INCONCLUSIVE",
                "reason": "input_ended_before_certification",
                "looks_used": n,
                "mean_difference": (total / n) if n else None,
                "lower": latest["lower"] if latest else None,
                "upper": latest["upper"] if latest else None,
            }
            break
        if kind != "observation":
            raise EvaluationError("records after start must be observation or end")
        if set(record) != {"type", "cluster_id", "paired_difference"}:
            raise EvaluationError("observation record must contain exactly type, cluster_id and paired_difference")
        cluster_id = _nonempty_string(record.get("cluster_id"), "cluster_id")
        if cluster_id in seen_clusters:
            raise EvaluationError("duplicate cluster_id; one row per independent source cluster")
        raw_difference = record.get("paired_difference")
        if not finite_number(raw_difference) or not -1.0 <= raw_difference <= 1.0:
            raise EvaluationError("paired_difference must be finite and in [-1, 1]")

        seen_clusters.add(cluster_id)
        value = float(raw_difference)
        n += 1
        total += value
        mean = total / n
        lower, upper = anytime_interval(n, mean, float(alpha))

        canonical = json.dumps(
            {"cluster_id": cluster_id, "paired_difference": value},
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=False,
        ).encode("utf-8")
        chain = hashlib.sha256(chain + canonical).digest()

        status = "CONTINUE"
        if lower > margin:
            status = "SUPERIOR"
        elif upper <= margin:
            status = "NOT_SUPERIOR"

        latest = {
            "looks_used": n,
            "mean_difference": mean,
            "lower": lower,
            "upper": upper,
        }
        emit({
            "type": "look",
            "status": status,
            **latest,
            "superiority_margin": float(margin),
            "alpha": float(alpha),
            "cluster_id": cluster_id,
            "transcript_chain_sha256": chain.hex(),
        })

        if status in {"SUPERIOR", "NOT_SUPERIOR"}:
            result = {
                "type": "decision",
                "status": status,
                "reason": "confidence_sequence_boundary_crossed",
                **latest,
            }
            break
        if n >= max_observations:
            result = {
                "type": "decision",
                "status": "INCONCLUSIVE",
                "reason": "maximum_observations_reached",
                **latest,
            }
            break
    else:
        result = {
            "type": "decision",
            "status": "INCONCLUSIVE",
            "reason": "input_exhausted",
            "looks_used": n,
            "mean_difference": (total / n) if n else None,
            "lower": latest["lower"] if latest else None,
            "upper": latest["upper"] if latest else None,
        }

    result.update({
        "benchmark_id": "av-anytime-valid-policy-tournament-v0",
        "metric_id": metric_id,
        "candidate_policy_id": candidate_id,
        "reference_policy_id": reference_id,
        "observations_consumed": n,
        "transcript_chain_sha256": chain.hex(),
        "method_id": "paired-hoeffding-confidence-sequence-v0",
        "alpha": float(alpha),
        "superiority_margin": float(margin),
        "max_observations": max_observations,
    })
    emit(result)
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--input",
        default="-",
        help="NDJSON input file, or - for stdin (default)",
    )
    args = parser.parse_args()
    try:
        if args.input == "-":
            run_stream(_read_ndjson(sys.stdin), lambda item: print(
                json.dumps(item, sort_keys=True), flush=True
            ))
        else:
            with open(args.input, "r", encoding="utf-8") as source:
                run_stream(_read_ndjson(source), lambda item: print(
                    json.dumps(item, sort_keys=True), flush=True
                ))
    except (OSError, EvaluationError) as exc:
        print(json.dumps({"type": "decision", "status": "INVALID", "error": str(exc)}), flush=True)
        raise SystemExit(2) from exc


if __name__ == "__main__":
    main()
