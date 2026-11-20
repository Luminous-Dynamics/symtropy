#!/usr/bin/env python3
"""Draft 2020-12 and semantic validator for thermal model evidence records."""
from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1] / "benchmarks" / "thermal-model-validation" / "v0.1"
SCHEMA_PATH = ROOT / "thermal_model_validation.schema.json"
EXAMPLE_PATH = ROOT / "examples" / "one_node_rc_heldout_synthetic.json"


def canonical_digest(value: Any) -> str:
    payload = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def unique(values: list[str]) -> bool:
    return len(values) == len(set(values))


def semantic_errors(record: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    dataset = record["dataset"]
    split = record["split"]
    fit = record["fit"]
    heldout = record["held_out_evaluation"]
    comparisons = record["model_comparisons"]
    evidence_ids = [item["evidence_id"] for item in record["evidence"]]
    traces = dataset["trace_catalog"]
    trace_ids = [trace["trace_id"] for trace in traces]
    trace_by_id = {trace["trace_id"]: trace for trace in traces}

    if not unique(trace_ids):
        errors.append("trace_catalog trace_id values must be unique")
    if not unique(evidence_ids):
        errors.append("evidence_id values must be unique")

    # Dataset digest binds the manifest identities and immutable trace digests.
    manifest = [
        {"trace_id": trace["trace_id"], "trace_digest": trace["trace_digest"]}
        for trace in sorted(traces, key=lambda item: item["trace_id"])
    ]
    if dataset["dataset_digest"] != canonical_digest(manifest):
        errors.append("dataset_digest does not match canonical trace manifest")

    partitions = {
        "training": split["training_trace_ids"],
        "validation": split["validation_trace_ids"],
        "held_out": split["held_out_trace_ids"],
    }
    flattened = [trace_id for values in partitions.values() for trace_id in values]
    if not unique(flattened):
        errors.append("trace IDs must appear in exactly one data partition")
    if set(flattened) != set(trace_ids):
        errors.append("data partitions must cover the trace catalog exactly once")
    for trace_id in flattened:
        if trace_id not in trace_by_id:
            errors.append(f"partition references unknown trace ID {trace_id}")

    expected_split_digest = canonical_digest({
        "training_trace_ids": split["training_trace_ids"],
        "validation_trace_ids": split["validation_trace_ids"],
        "held_out_trace_ids": split["held_out_trace_ids"],
    })
    if split["split_digest"] != expected_split_digest:
        errors.append("split_digest does not match locked partition IDs")

    training_manifest = [
        {"trace_id": trace_id, "trace_digest": trace_by_id[trace_id]["trace_digest"]}
        for trace_id in sorted(split["training_trace_ids"])
        if trace_id in trace_by_id
    ]
    if fit["training_data_digest"] != canonical_digest(training_manifest):
        errors.append("training_data_digest does not match training trace manifest")
    if not split["locked_before_fit"]:
        errors.append("split must be locked before fitting")

    for grouping_key in split["grouping_keys"]:
        groups = {
            partition: {
                trace_by_id[trace_id][grouping_key]
                for trace_id in ids
                if trace_id in trace_by_id
            }
            for partition, ids in partitions.items()
        }
        if groups["training"] & groups["validation"]:
            errors.append(f"{grouping_key} leakage between training and validation")
        if groups["training"] & groups["held_out"]:
            errors.append(f"{grouping_key} leakage between training and held-out")
        if groups["validation"] & groups["held_out"]:
            errors.append(f"{grouping_key} leakage between validation and held-out")

    evaluated = heldout["evaluated_trace_ids"]
    if not unique(evaluated):
        errors.append("held-out evaluated_trace_ids must be unique")
    if set(evaluated) != set(split["held_out_trace_ids"]):
        errors.append("held-out evaluation IDs must exactly match the locked held-out partition")

    per_trace_ids = [item["trace_id"] for item in heldout["per_trace"]]
    if not unique(per_trace_ids):
        errors.append("per_trace evaluation IDs must be unique")
    if set(per_trace_ids) != set(evaluated):
        errors.append("per_trace metrics must cover every held-out trace exactly once")

    comparisons_ids = [comparison["model_id"] for comparison in comparisons]
    if not unique(comparisons_ids):
        errors.append("model comparison model_id values must be unique")
    if record["model"]["model_id"] not in comparisons_ids:
        errors.append("model_comparisons must include the candidate model itself")

    expected_comparison_channels: set[str] | None = None
    for comparison in comparisons:
        if comparison["split_digest"] != split["split_digest"]:
            errors.append(f"model comparison {comparison['model_id']} uses a different split digest")
        if set(comparison["evaluated_trace_ids"]) != set(evaluated):
            errors.append(f"model comparison {comparison['model_id']} uses different held-out traces")
        if comparison["evaluation_method"] != heldout["evaluation_method"]:
            errors.append(f"model comparison {comparison['model_id']} uses a different evaluation method")
        if comparison["fit_budget_evaluations"] != fit["evaluation_budget"]:
            errors.append(f"model comparison {comparison['model_id']} uses a different declared fit budget")
        channel_ids = {metric["channel_id"] for metric in comparison["per_channel_metrics"]}
        if expected_comparison_channels is None:
            expected_comparison_channels = channel_ids
        elif channel_ids != expected_comparison_channels:
            errors.append("model comparisons must report the same channel metric set")

    for trace_result in heldout["per_trace"]:
        trace = trace_by_id.get(trace_result["trace_id"])
        if trace is None:
            continue
        channel_ids = [channel["channel_id"] for channel in trace["channels"]]
        result_channel_ids = [channel["channel_id"] for channel in trace_result["channels"]]
        if not unique(result_channel_ids):
            errors.append(f"trace {trace_result['trace_id']} repeats a held-out channel metric")
        if set(result_channel_ids) != set(channel_ids):
            errors.append(f"trace {trace_result['trace_id']} metrics do not match its declared sensor channels")

    if record["record_kind"] == "synthetic_analytic_fixture":
        if dataset["evidence_origin"] != "synthetic_generated":
            errors.append("synthetic_analytic_fixture must use synthetic_generated data origin")
        if "synthetic" not in record["claim_ceiling"].lower():
            errors.append("synthetic fixture claim_ceiling must explicitly say synthetic")
        for trace in traces:
            if any(channel["sensor_status"] != "synthetic" for channel in trace["channels"]):
                errors.append("synthetic fixture cannot contain physical sensor channels")

    if record["review"]["qualification_status"] == "passed":
        if record["review"]["independent_review"] != "yes":
            errors.append("passed qualification status requires independent review")
        if record["review"]["independent_reproduction"] != "yes":
            errors.append("passed qualification status requires independent reproduction")
        if fit["identifiability"]["status"] in {"not_identified", "not_evaluated"}:
            errors.append("passed qualification status requires an evaluated identifiable model")
    for trace in traces:
        for channel in trace["channels"]:
            if channel["sensor_status"] == "calibrated_physical_sensor" and not channel["calibration_reference"]:
                errors.append(f"physical channel {channel['channel_id']} lacks calibration reference")

    return errors


def validate(record: dict[str, Any], validator: Draft202012Validator) -> list[str]:
    schema_errors = sorted(
        validator.iter_errors(record),
        key=lambda error: tuple(str(part) for part in error.path),
    )
    messages = [
        "schema " + "/".join(str(part) for part in error.path) + ": " + error.message
        for error in schema_errors
    ]
    if not schema_errors:
        messages.extend(semantic_errors(record))
    return messages


def expect_rejected(name: str, original: dict[str, Any], validator: Draft202012Validator, mutate) -> None:
    candidate = copy.deepcopy(original)
    mutate(candidate)
    errors = validate(candidate, validator)
    if not errors:
        raise SystemExit(f"negative control unexpectedly accepted: {name}")
    print(f"rejected negative control: {name}")


def main() -> None:
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    example = json.loads(EXAMPLE_PATH.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())

    errors = validate(example, validator)
    if errors:
        raise SystemExit("\n".join(errors))
    print("positive fixture: schema and semantic validation passed")

    expect_rejected(
        "trace leakage between training and held-out article groups",
        example,
        validator,
        lambda item: item["dataset"]["trace_catalog"][3].update(article_group_id="synthetic-article-a"),
    )
    expect_rejected(
        "held-out trace reused in training",
        example,
        validator,
        lambda item: item["split"]["training_trace_ids"].append("run-repeated-mission-holdout-d"),
    )
    expect_rejected(
        "split manifest changed without digest update",
        example,
        validator,
        lambda item: item["split"].update(held_out_trace_ids=["run-partial-cooldown-c"]),
    )
    expect_rejected(
        "training data digest tampered",
        example,
        validator,
        lambda item: item["fit"].update(training_data_digest="sha256:" + "0" * 64),
    )
    expect_rejected(
        "comparison uses a different held-out set",
        example,
        validator,
        lambda item: item["model_comparisons"][1].update(evaluated_trace_ids=["run-hot-load-b"]),
    )
    expect_rejected(
        "comparison uses a different split digest",
        example,
        validator,
        lambda item: item["model_comparisons"][1].update(split_digest="sha256:" + "1" * 64),
    )
    expect_rejected(
        "per-trace metrics omit the held-out trace",
        example,
        validator,
        lambda item: item["held_out_evaluation"].update(per_trace=[]),
    )
    expect_rejected(
        "synthetic fixture disguises physical provenance",
        example,
        validator,
        lambda item: item["dataset"].update(evidence_origin="physical_measured"),
    )
    expect_rejected(
        "physical sensor lacks calibration certificate",
        example,
        validator,
        lambda item: item["dataset"]["trace_catalog"][0]["channels"][0].update(
            sensor_status="calibrated_physical_sensor",
            calibration_reference=None,
        ),
    )


if __name__ == "__main__":
    main()
