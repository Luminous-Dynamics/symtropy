#!/usr/bin/env python3
"""Conservative anytime-valid paired-mean evaluator (research benchmark tool).

The estimand is the mean of paired policy-score differences. Positive values
favor the candidate policy. Every independent sampling unit must contribute
one difference in [-1, 1]. The method is not valid for arbitrary dependent
clips, unbounded metrics, adaptively redefined estimands, or unregistered
multiple comparisons.

At look n, each tail receives alpha / (2*n*(n+1)). Hoeffding's inequality
gives radius sqrt(2*log(2*n*(n+1)/alpha)/n); summing the two tail budgets
over all n gives total error budget alpha. This is deliberately conservative.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
from typing import Any, Iterable

METHOD_ID = "paired-hoeffding-confidence-sequence-v0"


class EvaluationError(ValueError):
    """Invalid or unsupported evaluation input."""


def finite_number(value: Any) -> bool:
    return (
        not isinstance(value, bool)
        and isinstance(value, (int, float))
        and math.isfinite(float(value))
    )


def anytime_interval(n: int, mean: float, alpha: float) -> tuple[float, float]:
    """Two-sided time-uniform Hoeffding interval for a mean in [-1, 1]."""
    if isinstance(n, bool) or not isinstance(n, int) or n < 1:
        raise EvaluationError("n must be an integer >= 1")
    if not finite_number(alpha) or not 0.0 < alpha < 1.0:
        raise EvaluationError("alpha must be finite and strictly between 0 and 1")
    if not finite_number(mean) or not -1.0 <= mean <= 1.0:
        raise EvaluationError("mean must be finite and in [-1, 1]")
    radius = math.sqrt(2.0 * math.log(2.0 * n * (n + 1) / alpha) / n)
    return max(-1.0, mean - radius), min(1.0, mean + radius)


def evaluate_paired_differences(
    differences: Iterable[float], *, alpha: float = 0.05, margin: float = 0.0
) -> dict[str, Any]:
    """Stop when superiority over margin or non-superiority is certified."""
    if not finite_number(alpha) or not 0.0 < alpha < 1.0:
        raise EvaluationError("alpha must be finite and strictly between 0 and 1")
    if not finite_number(margin) or not -1.0 <= margin <= 1.0:
        raise EvaluationError("margin must be finite and in [-1, 1]")

    total = 0.0
    n = 0
    latest: tuple[float, float, float] | None = None
    for raw in differences:
        if not finite_number(raw):
            raise EvaluationError("paired difference must be a finite numeric value")
        value = float(raw)
        if not -1.0 <= value <= 1.0:
            raise EvaluationError("every paired difference must be in [-1, 1]")
        n += 1
        total += value
        mean = total / n
        lower, upper = anytime_interval(n, mean, alpha)
        latest = (mean, lower, upper)
        if lower > margin:
            status = "SUPERIOR"
        elif upper <= margin:
            status = "NOT_SUPERIOR"
        else:
            continue
        return {
            "method_id": METHOD_ID,
            "status": status,
            "looks_used": n,
            "mean_difference": mean,
            "lower": lower,
            "upper": upper,
            "superiority_margin": float(margin),
            "alpha": float(alpha),
        }

    if latest is None:
        raise EvaluationError("at least one independent paired observation is required")
    mean, lower, upper = latest
    return {
        "method_id": METHOD_ID,
        "status": "INCONCLUSIVE",
        "looks_used": n,
        "mean_difference": mean,
        "lower": lower,
        "upper": upper,
        "superiority_margin": float(margin),
        "alpha": float(alpha),
    }


def evaluate_document(document: dict[str, Any]) -> dict[str, Any]:
    """Validate the v1 input envelope and return a replayable score record."""
    version = document.get("schema_version")
    if isinstance(version, bool) or version != 1:
        raise EvaluationError("schema_version must be integer 1")
    if document.get("sampling_unit") != "independent_source_cluster":
        raise EvaluationError("v1 requires sampling_unit=independent_source_cluster")
    rows = document.get("observations")
    if not isinstance(rows, list) or not rows:
        raise EvaluationError("observations must be a non-empty array")

    cluster_ids: list[str] = []
    seen_clusters: set[str] = set()
    differences: list[float] = []
    for row in rows:
        if not isinstance(row, dict):
            raise EvaluationError("each observation must be an object")
        cluster_id = row.get("cluster_id")
        if not isinstance(cluster_id, str) or not cluster_id:
            raise EvaluationError("each observation requires a non-empty cluster_id")
        if cluster_id in seen_clusters:
            raise EvaluationError("cluster_id must be unique; clips are not independent units")
        seen_clusters.add(cluster_id)
        cluster_ids.append(cluster_id)
        differences.append(row.get("paired_difference"))

    result = evaluate_paired_differences(
        differences,
        alpha=document.get("alpha", 0.05),
        margin=document.get("superiority_margin", 0.0),
    )
    consumed_ids = cluster_ids[: result["looks_used"]]
    result.update({
        "benchmark_id": "av-anytime-valid-policy-tournament-v0",
        "metric_id": document.get("metric_id", "unspecified"),
        "candidate_policy_id": document.get("candidate_policy_id", "candidate"),
        "reference_policy_id": document.get("reference_policy_id", "reference"),
        "observations_provided": len(cluster_ids),
        "independent_cluster_count": len(consumed_ids),
        "consumed_cluster_ids_sha256": hashlib.sha256(
            "\n".join(consumed_ids).encode("utf-8")
        ).hexdigest(),
        "assumptions": [
            "paired differences are bounded in [-1, 1]",
            "each consumed row is one independent source cluster",
            "estimand and sampling law were frozen before confirmatory evaluation",
            "alpha and superiority margin were preregistered",
            "this result is one estimand only; familywise multiplicity is not handled",
        ],
    })
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        document = json.loads(args.input.read_text(encoding="utf-8"))
        if not isinstance(document, dict):
            raise EvaluationError("input root must be an object")
        result = evaluate_document(document)
        rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.write_text(rendered, encoding="utf-8")
        else:
            print(rendered, end="")
    except (OSError, json.JSONDecodeError, EvaluationError) as exc:
        parser.error(str(exc))


if __name__ == "__main__":
    main()
