#!/usr/bin/env python3
"""Validate the CivBench task-evaluation sidecar against a Humanity Atlas run.

This is a contract/integration test with synthetic fixtures. It does not assess
empirical validity, publisher identity, or the quality of a simulation model.
"""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path
from typing import Any

import rfc8785
from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "docs/research/fixtures/civbench_atlas_sidecar_v0_1"
SIDECAR_SCHEMA_PATH = ROOT / "docs/research/schemas/CIVBENCH_TASK_EVALUATION_V0_1.schema.json"
RUN_SCHEMA_PATH = FIXTURE / "humanity_atlas_run_v0_1.schema.snapshot.json"

# Git blob SHA from PR #1571 at the exact source commit documented below.
PINNED_RUN_SCHEMA_BLOB_SHA1 = "429c5bd88afbf012d08c3d80ba61a6f09de95d72"
PINNED_RUN_SCHEMA_SOURCE_COMMIT = "7f0128bca016f324faaf9041e9c99b3e3f65ec03"

RUN_CORE_KEYS = (
    "schema_version",
    "run_id",
    "scenario_id",
    "inputs",
    "engine",
    "run_status",
    "output_classification",
    "deterministic",
    "seed",
    "replicates",
)


class ContractFailure(ValueError):
    """A sidecar/run integration contract has been violated."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractFailure(message)


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as stream:
        value = json.load(stream)
    require(isinstance(value, dict), f"{path.name}: expected a JSON object")
    return value


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def git_blob_sha1(data: bytes) -> str:
    header = b"blob " + str(len(data)).encode("ascii") + b"\0"
    return hashlib.sha1(header + data).hexdigest()


def run_core_projection(envelope: dict[str, Any]) -> dict[str, Any]:
    missing = [key for key in RUN_CORE_KEYS if key not in envelope]
    require(not missing, f"run envelope is missing run-core keys: {missing}")
    # Explicit field allowlist: outputs, validation, warnings and artifacts are
    # intentionally excluded to avoid a circular sidecar<->envelope hash.
    return {key: envelope[key] for key in RUN_CORE_KEYS}


def run_core_sha256(envelope: dict[str, Any]) -> str:
    return sha256_bytes(rfc8785.dumps(run_core_projection(envelope)))


def artifact_path(base: Path, artifact_ref: str) -> Path:
    relative = Path(artifact_ref)
    require(not relative.is_absolute(), f"artifact_ref must be relative: {artifact_ref}")
    require(".." not in relative.parts, f"artifact_ref path traversal rejected: {artifact_ref}")
    resolved = (base / relative).resolve()
    require(resolved.is_relative_to(base.resolve()), f"artifact_ref escapes fixture directory: {artifact_ref}")
    require(resolved.is_file(), f"artifact file not found: {artifact_ref}")
    return resolved


def validate_run_schema(envelope: dict[str, Any], run_schema: dict[str, Any]) -> None:
    Draft202012Validator.check_schema(run_schema)
    errors = sorted(Draft202012Validator(run_schema).iter_errors(envelope), key=lambda error: list(error.absolute_path))
    if errors:
        first = errors[0]
        location = "/".join(str(item) for item in first.absolute_path) or "<root>"
        raise ContractFailure(f"Humanity Atlas Run v0.1 schema failed at {location}: {first.message}")


def validate_sidecar_schema(sidecar: dict[str, Any], schema: dict[str, Any]) -> None:
    Draft202012Validator.check_schema(schema)
    errors = sorted(Draft202012Validator(schema).iter_errors(sidecar), key=lambda error: list(error.absolute_path))
    if errors:
        first = errors[0]
        location = "/".join(str(item) for item in first.absolute_path) or "<root>"
        raise ContractFailure(f"CivBench sidecar schema failed at {location}: {first.message}")


def validate_cross_fields(
    sidecar: dict[str, Any],
    envelope: dict[str, Any],
    fixture_dir: Path,
) -> None:
    binding = sidecar["run_binding"]
    require(binding["run_id"] == envelope["run_id"], "sidecar run_id does not match Humanity Atlas run_id")
    require(
        binding["run_core_projection"] == "humanity-atlas-run-core.v1",
        "unsupported Humanity Atlas run-core projection version",
    )
    require(binding["canonicalization"] == "RFC8785", "run-core canonicalization must be RFC8785")
    computed_core = run_core_sha256(envelope)
    require(
        binding["run_core_sha256"] == computed_core,
        "run-core digest mismatch: sidecar is not bound to this exact run identity",
    )

    model = sidecar["model_identity"]
    engine = envelope["engine"]
    for key in ("adapter", "engine_version", "model_sha256", "source_revision"):
        require(model[key] == engine[key], f"sidecar model_identity.{key} does not match run engine.{key}")

    dataset_by_id = {dataset["dataset_id"]: dataset for dataset in sidecar["datasets"]}
    input_dataset_sha = envelope["inputs"]["dataset_sha256"]
    require(input_dataset_sha is not None, "dataset_sha256 is null; the synthetic contract fixture expects an identified dataset")
    require(
        any(dataset["content_sha256"] == input_dataset_sha for dataset in dataset_by_id.values()),
        "no sidecar dataset digest matches Humanity Atlas inputs.dataset_sha256",
    )

    split_ids = {dataset["split_id"] for dataset in sidecar["datasets"]}
    declared_strata = set(sidecar["scope"]["declared_strata"])
    metric_by_id = {metric["metric_id"]: metric for metric in sidecar["metrics"]}
    require(len(metric_by_id) == len(sidecar["metrics"]), "duplicate metric_id in sidecar")

    for metric in sidecar["metrics"]:
        require(metric["stratum_id"] in declared_strata, f"metric {metric['metric_id']} uses an undeclared stratum")
        require(metric["sample_count"] <= envelope["replicates"], f"metric {metric['metric_id']} sample_count exceeds run replicates")
        if metric["sample_unit"] == "simulation_replication":
            require(metric["sample_count"] == envelope["replicates"], f"metric {metric['metric_id']} does not account for all run replicates")
        if metric["value"] is None:
            require(
                metric["uncertainty"]["status"] != "estimated",
                f"metric {metric['metric_id']} cannot claim estimated uncertainty without a reported value",
            )
        for artifact_id in metric["result_artifact_ids"]:
            require(
                any(artifact["artifact_id"] == artifact_id for artifact in sidecar["artifacts"]),
                f"metric {metric['metric_id']} references unknown result artifact {artifact_id}",
            )

    for comparison in sidecar["baseline_comparisons"]:
        candidate = metric_by_id.get(comparison["candidate_metric_id"])
        require(candidate is not None, f"baseline comparison references unknown candidate metric {comparison['candidate_metric_id']}")
        require(candidate["unit"] == comparison["unit"], "baseline comparison unit differs from candidate metric unit")
        require(candidate["value"] == comparison["candidate_value"], "baseline comparison candidate_value differs from scored metric")
        require(comparison["dataset_split_id"] in split_ids, "baseline comparison uses an unknown dataset split")
        require(comparison["metric_version"] == sidecar["task"]["task_version"], "baseline comparison metric version is not aligned to task version")
        require(comparison["same_task_and_split"] is True, "baseline comparison must declare same-task/same-split protocol")

    for run_output in envelope["outputs"]:
        metric = metric_by_id.get(run_output["metric_id"])
        if metric is not None:
            require(metric["unit"] == run_output["unit"], f"simple run output unit mismatch for {run_output['metric_id']}")
            require(metric["statistic"] == run_output["statistic"], f"simple run output statistic mismatch for {run_output['metric_id']}")
            require(metric["value"] == run_output["value"], f"simple run output value mismatch for {run_output['metric_id']}")
            require(metric["sample_count"] == run_output["sample_count"], f"simple run output sample count mismatch for {run_output['metric_id']}")

    # Verify every artifact payload referenced by the sidecar and ensure its
    # digest is also available through the run envelope's artifact list.
    run_artifacts = envelope.get("artifacts", [])
    for artifact in sidecar["artifacts"]:
        path = artifact_path(fixture_dir, artifact["artifact_ref"])
        digest = sha256_file(path)
        require(digest == artifact["artifact_sha256"], f"sidecar artifact digest mismatch: {artifact['artifact_ref']}")
        envelope_artifact = next(
            (
                item for item in run_artifacts
                if item["artifact_id"] == artifact["artifact_id"]
                and item["artifact_sha256"] == artifact["artifact_sha256"]
            ),
            None,
        )
        require(envelope_artifact is not None, f"sidecar artifact {artifact['artifact_id']} is not bound in Humanity Atlas artifacts[]")

    sidecar_path = fixture_dir / "civbench_task_evaluation.json"
    sidecar_digest = sha256_file(sidecar_path)
    envelope_sidecar = next(
        (
            item for item in run_artifacts
            if item["role"] == "civbench_task_evaluation"
            and item["artifact_ref"] == sidecar_path.name
        ),
        None,
    )
    require(envelope_sidecar is not None, "Humanity Atlas artifacts[] is missing the CivBench sidecar")
    require(
        envelope_sidecar["artifact_sha256"] == sidecar_digest,
        "Humanity Atlas sidecar artifact digest does not match exact sidecar bytes",
    )

    protocol_path = fixture_dir / "benchmark_protocol.json"
    require(
        sha256_file(protocol_path) == sidecar["protocol"]["protocol_sha256"],
        "benchmark protocol digest mismatch",
    )
    dataset_path = fixture_dir / "synthetic_dataset.csv"
    require(
        sha256_file(dataset_path) == input_dataset_sha,
        "input dataset bytes do not match Humanity Atlas inputs.dataset_sha256",
    )

    # Never silently convert missingness to zero or introduce a cross-metric score.
    require("overall_score" not in sidecar, "universal aggregate score is prohibited by this sidecar contract")
    require(
        sidecar["assessment"]["claim_ceiling"] == "synthetic_test_only",
        "synthetic fixture must retain its synthetic-only claim ceiling",
    )


def expect_rejection(label: str, callback) -> None:
    try:
        callback()
    except ContractFailure:
        return
    raise ContractFailure(f"negative control did not reject tampering: {label}")


def main() -> None:
    schema = read_json(SIDECAR_SCHEMA_PATH)
    sidecar = read_json(FIXTURE / "civbench_task_evaluation.json")
    envelope = read_json(FIXTURE / "humanity_atlas_run.json")
    run_schema = read_json(RUN_SCHEMA_PATH)

    run_schema_bytes = RUN_SCHEMA_PATH.read_bytes()
    actual_blob = git_blob_sha1(run_schema_bytes)
    require(
        actual_blob == PINNED_RUN_SCHEMA_BLOB_SHA1,
        "pinned Humanity Atlas Run v0.1 schema snapshot differs from PR #1571 source commit "
        + PINNED_RUN_SCHEMA_SOURCE_COMMIT,
    )

    validate_run_schema(envelope, run_schema)
    validate_sidecar_schema(sidecar, schema)
    validate_cross_fields(sidecar, envelope, FIXTURE)

    # Negative control 1: changing the run ID invalidates the sidecar binding.
    changed_run = copy.deepcopy(envelope)
    changed_run["run_id"] += "-tampered"
    expect_rejection(
        "run_id substitution",
        lambda: validate_cross_fields(sidecar, changed_run, FIXTURE),
    )

    # Negative control 2: changing a seed changes the canonical run-core digest.
    changed_seed = copy.deepcopy(envelope)
    changed_seed["seed"] += 1
    expect_rejection(
        "seed substitution",
        lambda: validate_cross_fields(sidecar, changed_seed, FIXTURE),
    )

    # Negative control 3: an artifact hash substitution is rejected by the sidecar schema.
    changed_sidecar = copy.deepcopy(sidecar)
    changed_sidecar["artifacts"][0]["artifact_sha256"] = "0" * 64
    expect_rejection(
        "artifact digest substitution",
        lambda: validate_cross_fields(changed_sidecar, envelope, FIXTURE),
    )

    # Negative control 4: a missing stratum must not be treated as an observed zero.
    missing_stratum = copy.deepcopy(sidecar)
    missing_stratum["metrics"][0]["stratum_id"] = "unregistered_stratum"
    expect_rejection(
        "unknown metric stratum",
        lambda: validate_cross_fields(missing_stratum, envelope, FIXTURE),
    )

    # Negative control 5: same-task/same-split baseline claims are mandatory.
    wrong_split = copy.deepcopy(sidecar)
    wrong_split["baseline_comparisons"][0]["dataset_split_id"] = "other-split"
    expect_rejection(
        "baseline split mismatch",
        lambda: validate_cross_fields(wrong_split, envelope, FIXTURE),
    )

    print("CIVBENCH-ATLAS-001 PASS")
    print(f"run_id={envelope['run_id']}")
    print(f"run_core_sha256={run_core_sha256(envelope)}")
    print(f"sidecar_sha256={sha256_file(FIXTURE / 'civbench_task_evaluation.json')}")
    print("run_schema_snapshot=byte_identical_to_pr_1571")
    print("cross_field_negative_controls=5/5_rejected")
    print("claim_ceiling=synthetic_test_only")


if __name__ == "__main__":
    try:
        main()
    except ContractFailure as error:
        raise SystemExit(f"CONTRACT FAILURE: {error}") from error
