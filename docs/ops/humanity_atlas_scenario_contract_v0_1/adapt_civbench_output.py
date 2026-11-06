#!/usr/bin/env python3
"""Adapt CivBench domain CSV reports into the existing Humanity Atlas run v0.1 envelope.

This is a transport/contract adapter, not a simulator, scientific validator,
source authenticator, or calibration gate. The full domain CSV is retained as
a content-addressed artifact; scalar outputs are only an indexed metric view.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import os
import re
import sys
import tempfile
from pathlib import Path
from typing import Any, Iterable

RUN_SCHEMA_VERSION = "humanity-atlas.run.v0.1"
ADAPTER_ID = "civbench-domain-csv-humanity-atlas-adapter"
MAX_SOURCE_BYTES = 64 * 1024 * 1024
MAX_OUTPUT_BYTES = 4 * 1024 * 1024
MAX_OUTPUT_METRICS = 100_000
SHA256_RE = re.compile(r"^[a-f0-9]{64}$")

SERVICE_HEADER = [
    "record_type", "scenario_id", "node_id", "service_type", "tick_index",
    "own_availability", "effective_service", "minimum_service",
    "beneficiary_demand_weight", "deficit_area_service_ticks", "ticks_below_50",
    "ticks_below_90", "recovery_after_ticks", "baseline_service_deficit_area",
    "outage_service_deficit_area", "marginal_service_deficit_area",
    "demand_weighted_service_deficit_area",
]
KNOWLEDGE_HEADER = [
    "record_type", "item_id", "scope", "event_detail", "epistemic_status",
    "semantic_family_id", "source_evidence_family_count", "carrier_state_count",
    "transmission_events", "delivered_transmissions", "acquisition_count",
    "comprehension_count", "demonstration_assessed_count",
    "demonstration_success_count", "retention_assessed_count", "retained_count",
    "independent_teacher_count", "refuted_knowledge_acquisition_count",
    "demonstrated_generation_count", "longest_consecutive_demonstrated_generations",
    "verified_archive_copy_count", "independent_archive_failure_domain_count",
    "recovery_surviving_verified_copy_count", "recoverable",
]


class AdapterError(ValueError):
    """Input does not meet the fail-closed CivBench adapter contract."""


def _reject_constant(token: str) -> None:
    raise AdapterError(f"non-standard JSON numeric constant: {token}")


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise AdapterError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def read_bounded(path: Path, limit: int) -> bytes:
    try:
        stat_size = path.stat().st_size
    except OSError as exc:
        raise AdapterError(f"cannot stat {path}: {exc}") from exc
    if stat_size < 0 or stat_size > limit:
        raise AdapterError(f"{path} exceeds the {limit}-byte input limit")
    try:
        with path.open("rb") as stream:
            raw = stream.read(limit + 1)
    except OSError as exc:
        raise AdapterError(f"cannot read {path}: {exc}") from exc
    if len(raw) > limit:
        raise AdapterError(f"{path} grew beyond the {limit}-byte input limit")
    return raw


def strict_json(raw: bytes, label: str) -> dict[str, Any]:
    try:
        parsed = json.loads(
            raw.decode("utf-8", errors="strict"),
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise AdapterError(f"{label} is not strict UTF-8 JSON: {exc}") from exc
    if not isinstance(parsed, dict):
        raise AdapterError(f"{label} must be a JSON object")
    _check_finite(parsed, label)
    return parsed


def _check_finite(value: Any, label: str) -> None:
    if isinstance(value, float) and not math.isfinite(value):
        raise AdapterError(f"{label} contains a non-finite number")
    if isinstance(value, dict):
        for key, child in value.items():
            _check_finite(child, f"{label}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _check_finite(child, f"{label}[{index}]")


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def canonical_json(value: Any) -> bytes:
    try:
        return (
            json.dumps(
                value,
                sort_keys=True,
                separators=(",", ":"),
                ensure_ascii=False,
                allow_nan=False,
            )
            + "\n"
        ).encode("utf-8")
    except (TypeError, ValueError) as exc:
        raise AdapterError(f"cannot serialize canonical JSON: {exc}") from exc


def _read_csv(raw: bytes, track: str) -> list[dict[str, str]]:
    try:
        text = raw.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        raise AdapterError(f"CSV is not UTF-8: {exc}") from exc
    reader = csv.DictReader(text.splitlines())
    header = reader.fieldnames
    expected = SERVICE_HEADER if track == "service-cascade" else KNOWLEDGE_HEADER
    if header is None or header != expected:
        raise AdapterError(
            f"{track} CSV header does not match the frozen v0.1 schema; "
            f"expected {expected!r}, got {header!r}"
        )
    rows: list[dict[str, str]] = []
    for line_number, row in enumerate(reader, start=2):
        if None in row:
            raise AdapterError(f"CSV line {line_number} has more columns than the header")
        if any(value is None for value in row.values()):
            raise AdapterError(f"CSV line {line_number} has fewer columns than the header")
        rows.append({key: str(value) for key, value in row.items()})
    if not rows:
        raise AdapterError("domain CSV has no data rows")
    return rows


def _number(row: dict[str, str], key: str, *, allow_empty: bool = False) -> float | None:
    raw = row.get(key, "")
    if raw == "" and allow_empty:
        return None
    try:
        value = float(raw)
    except (TypeError, ValueError) as exc:
        raise AdapterError(f"{key} must be a finite numeric value, got {raw!r}") from exc
    if not math.isfinite(value):
        raise AdapterError(f"{key} must be finite, got {raw!r}")
    return value


def _integer(row: dict[str, str], key: str, *, allow_empty: bool = False) -> int | None:
    value = _number(row, key, allow_empty=allow_empty)
    if value is None:
        return None
    if not value.is_integer():
        raise AdapterError(f"{key} must be an integer, got {value}")
    result = int(value)
    if result < -100_000 or result > 100_000:
        raise AdapterError(f"{key} is outside the run-v0.1 year-coordinate bounds")
    return result


def _metric(
    metric_id: str,
    year: int,
    unit: str,
    value: float,
    *,
    sample_count: int = 1,
) -> dict[str, Any]:
    if not metric_id or len(metric_id) > 200:
        raise AdapterError(f"metric ID is empty or too long: {metric_id!r}")
    if not unit or len(unit) > 80:
        raise AdapterError(f"metric unit is empty or too long: {unit!r}")
    if not math.isfinite(value):
        raise AdapterError(f"metric {metric_id} has a non-finite value")
    if year < -100_000 or year > 100_000:
        raise AdapterError(f"metric {metric_id} has an out-of-range time coordinate")
    return {
        "metric_id": metric_id,
        "year": year,
        "unit": unit,
        "statistic": "value",
        "value": value,
        "sample_count": sample_count,
        "time_basis": "scenario_relative_year",
    }


def _service_outputs(rows: list[dict[str, str]], scenario_id: str) -> list[dict[str, Any]]:
    outputs: list[dict[str, Any]] = []
    seen_scenario_ids = {row["scenario_id"] for row in rows}
    if seen_scenario_ids != {scenario_id}:
        raise AdapterError(
            f"service-cascade scenario IDs {seen_scenario_ids!r} do not match "
            f"scenario descriptor {scenario_id!r}"
        )
    supported = {"tick", "node_summary", "criticality", "scenario_summary"}
    record_types = {row["record_type"] for row in rows}
    if record_types - supported:
        raise AdapterError(f"unsupported service-cascade record types: {record_types - supported}")
    if not {"tick", "node_summary", "criticality", "scenario_summary"}.issubset(record_types):
        raise AdapterError("service-cascade report is missing a required record type")

    tick_keys: set[tuple[str, int]] = set()
    for row in rows:
        kind = row["record_type"]
        node_id = row["node_id"]
        if kind == "tick":
            year = _integer(row, "tick_index")
            assert year is not None
            key = (node_id, year)
            if key in tick_keys:
                raise AdapterError(f"duplicate service-cascade tick record: {key}")
            tick_keys.add(key)
            for field in ("own_availability", "effective_service"):
                value = _number(row, field)
                assert value is not None
                outputs.append(_metric(f"service.{node_id}.{field}", year, "fraction", value))
        elif kind == "node_summary":
            for field, unit in (
                ("minimum_service", "fraction"),
                ("deficit_area_service_ticks", "service_ticks"),
                ("ticks_below_50", "ticks"),
                ("ticks_below_90", "ticks"),
            ):
                value = _number(row, field)
                assert value is not None
                outputs.append(_metric(f"service.{node_id}.{field}", 0, unit, value))
            recovery = _integer(row, "recovery_after_ticks", allow_empty=True)
            if recovery is not None:
                outputs.append(_metric(f"service.{node_id}.recovery_after_ticks", 0, "ticks", float(recovery)))
        elif kind == "criticality":
            for field in (
                "baseline_service_deficit_area",
                "outage_service_deficit_area",
                "marginal_service_deficit_area",
            ):
                value = _number(row, field)
                assert value is not None
                outputs.append(
                    _metric(f"criticality.{node_id}.{field}", 0, "demand_weighted_service_ticks", value)
                )
        elif kind == "scenario_summary":
            value = _number(row, "demand_weighted_service_deficit_area")
            assert value is not None
            outputs.append(
                _metric(
                    "scenario.demand_weighted_service_deficit_area",
                    0,
                    "demand_weighted_service_ticks",
                    value,
                )
            )
    if not tick_keys:
        raise AdapterError("service-cascade report contains no tick-level service observations")
    return outputs


def _knowledge_outputs(rows: list[dict[str, str]], scenario_id: str) -> list[dict[str, Any]]:
    # Knowledge artifacts do not contain a separate scenario_id field. The
    # scenario descriptor provides the stable run identity; the CSV artifact
    # retains each item, epistemic label, scope and participant transmission.
    del scenario_id
    outputs: list[dict[str, Any]] = []
    supported = {"item_summary", "transmission_event", "archive_recovery"}
    record_types = {row["record_type"] for row in rows}
    if record_types - supported:
        raise AdapterError(f"unsupported knowledge record types: {record_types - supported}")
    if not {"item_summary", "transmission_event", "archive_recovery"}.issubset(record_types):
        raise AdapterError("knowledge report is missing a required record type")

    item_ids: set[str] = set()
    summary_fields = (
        "source_evidence_family_count",
        "carrier_state_count",
        "transmission_events",
        "delivered_transmissions",
        "acquisition_count",
        "comprehension_count",
        "demonstration_assessed_count",
        "demonstration_success_count",
        "retention_assessed_count",
        "retained_count",
        "independent_teacher_count",
        "refuted_knowledge_acquisition_count",
        "demonstrated_generation_count",
        "longest_consecutive_demonstrated_generations",
        "verified_archive_copy_count",
        "independent_archive_failure_domain_count",
    )
    for row in rows:
        kind = row["record_type"]
        item_id = row["item_id"]
        if kind == "item_summary":
            if item_id in item_ids:
                raise AdapterError(f"duplicate knowledge item summary: {item_id}")
            item_ids.add(item_id)
            if row["scope"] != "aggregate":
                raise AdapterError(f"knowledge summary for {item_id} has unexpected scope {row['scope']!r}")
            status = row["epistemic_status"]
            if status not in {"verified", "provisional", "disputed", "refuted"}:
                raise AdapterError(f"knowledge item {item_id} has invalid epistemic status {status!r}")
            for field in summary_fields:
                value = _number(row, field)
                assert value is not None
                unit = "generations" if "generation" in field else "count"
                outputs.append(_metric(f"knowledge.{item_id}.{field}", 0, unit, value))
        elif kind == "transmission_event":
            delivered = row["delivered_transmissions"]
            if delivered not in {"true", "false"}:
                raise AdapterError(f"transmission for {item_id} has invalid delivered flag {delivered!r}")
            value = 1.0 if delivered == "true" else 0.0
            event_detail = row["event_detail"]
            method = row["scope"]
            outputs.append(
                _metric(
                    f"knowledge.{item_id}.transmission.{method}.{event_detail}.delivered",
                    0,
                    "boolean",
                    value,
                )
            )
        elif kind == "archive_recovery":
            scope = row["scope"]
            count = _number(row, "recovery_surviving_verified_copy_count")
            recoverable = row["recoverable"]
            if recoverable not in {"true", "false"}:
                raise AdapterError(f"archive recovery for {item_id} has invalid boolean {recoverable!r}")
            assert count is not None
            outputs.append(
                _metric(f"knowledge.{item_id}.archive.{scope}.surviving_verified_copies", 0, "count", count)
            )
            outputs.append(
                _metric(
                    f"knowledge.{item_id}.archive.{scope}.recoverable",
                    0,
                    "boolean",
                    1.0 if recoverable == "true" else 0.0,
                )
            )
    if not item_ids:
        raise AdapterError("knowledge report contains no item summaries")
    return outputs


def _model_manifest(paths: Iterable[Path]) -> tuple[str, bytes]:
    entries: list[dict[str, Any]] = []
    for path in sorted(paths, key=lambda item: item.as_posix()):
        if path.is_absolute() or ".." in path.parts:
            raise AdapterError(f"model source path must be repository-relative: {path}")
        if path.is_symlink():
            raise AdapterError(f"model source path must not be a symlink: {path}")
        if not path.is_file():
            raise AdapterError(f"model source path is not a regular file: {path}")
        raw = read_bounded(path, MAX_SOURCE_BYTES)
        entries.append({
            "path": path.as_posix(),
            "size_bytes": len(raw),
            "sha256": sha256(raw),
        })
    if not entries:
        raise AdapterError("at least one --model-source file is required")
    manifest = canonical_json({"files": entries})
    return sha256(manifest), manifest


def build_envelope(
    *,
    track: str,
    csv_bytes: bytes,
    csv_ref: str,
    scenario_bytes: bytes,
    scenario_ref: str,
    dataset_bytes: bytes,
    dataset_ref: str,
    model_sha: str,
    model_manifest: bytes,
    model_manifest_ref: str,
    source_revision: str | None,
    replay_bytes: bytes | None,
) -> dict[str, Any]:
    scenario = strict_json(scenario_bytes, "scenario document")
    dataset = strict_json(dataset_bytes, "dataset fixture")
    if not SHA256_RE.fullmatch(model_sha):
        raise AdapterError("model source digest is malformed")
    try:
        import jsonschema  # type: ignore
    except ImportError as exc:
        raise AdapterError("jsonschema==4.26.0 is required for schema validation") from exc

    here = Path(__file__).resolve().parent
    scenario_schema = strict_json(
        read_bounded(here / "schemas" / "scenario-v0.1.schema.json", MAX_OUTPUT_BYTES),
        "scenario-v0.1 schema",
    )
    jsonschema.Draft202012Validator.check_schema(scenario_schema)
    try:
        jsonschema.Draft202012Validator(scenario_schema).validate(scenario)
    except jsonschema.ValidationError as exc:
        raise AdapterError(f"scenario-v0.1 schema validation failed: {exc.message}") from exc

    # The dataset bytes are content-addressed independently and must match the
    # scenario's baseline digest exactly.
    dataset_sha = sha256(dataset_bytes)
    baseline = scenario.get("baseline")
    if not isinstance(baseline, dict) or baseline.get("dataset_sha256") != dataset_sha:
        raise AdapterError("scenario baseline.dataset_sha256 does not match exact dataset bytes")
    if not isinstance(dataset, dict):
        raise AdapterError("dataset fixture must be an object")

    engine = scenario.get("engine")
    if not isinstance(engine, dict):
        raise AdapterError("scenario engine is missing")
    if engine.get("model_sha256") != model_sha:
        raise AdapterError("scenario engine.model_sha256 does not match exact model-source manifest")
    if track not in {"service-cascade", "knowledge-resilience"}:
        raise AdapterError(f"unsupported CivBench track: {track}")
    rows = _read_csv(csv_bytes, track)
    scenario_id = scenario.get("scenario_id")
    if not isinstance(scenario_id, str):
        raise AdapterError("scenario_id is missing from scenario document")
    outputs = (
        _service_outputs(rows, scenario_id)
        if track == "service-cascade"
        else _knowledge_outputs(rows, scenario_id)
    )
    if not outputs or len(outputs) > MAX_OUTPUT_METRICS:
        raise AdapterError(f"output metric count is invalid: {len(outputs)}")

    horizon = scenario.get("horizon")
    if not isinstance(horizon, dict):
        raise AdapterError("scenario horizon is missing")
    start_year = horizon.get("start_year")
    end_year = horizon.get("end_year")
    if not isinstance(start_year, int) or not isinstance(end_year, int):
        raise AdapterError("scenario horizon years must be integers")
    for output in outputs:
        if output["year"] < start_year or output["year"] > end_year:
            raise AdapterError(
                f"output {output['metric_id']} year {output['year']} falls outside scenario horizon"
            )

    csv_sha = sha256(csv_bytes)
    scenario_sha = sha256(scenario_bytes)
    model_manifest_sha = sha256(model_manifest)
    artifacts = [
        {
            "artifact_id": "civbench-domain-csv",
            "role": "complete-domain-benchmark-report",
            "media_type": "text/csv",
            "artifact_ref": csv_ref,
            "artifact_sha256": csv_sha,
            "json_pointer": None,
        },
        {
            "artifact_id": "civbench-scenario-v0.1",
            "role": "portable-scenario-input",
            "media_type": "application/json",
            "artifact_ref": scenario_ref,
            "artifact_sha256": scenario_sha,
            "json_pointer": None,
        },
        {
            "artifact_id": "civbench-dataset-fixture",
            "role": "synthetic-dataset-fixture",
            "media_type": "application/json",
            "artifact_ref": dataset_ref,
            "artifact_sha256": dataset_sha,
            "json_pointer": None,
        },
        {
            "artifact_id": "civbench-model-source-manifest",
            "role": "model-source-digest-manifest",
            "media_type": "application/json",
            "artifact_ref": model_manifest_ref,
            "artifact_sha256": model_manifest_sha,
            "json_pointer": None,
        },
    ]

    declared_deterministic = engine.get("deterministic")
    if not isinstance(declared_deterministic, bool):
        raise AdapterError("scenario engine deterministic must be boolean")
    deterministic: bool | None = False if not declared_deterministic else None
    validation_evidence: list[dict[str, str]] = []
    warnings = [
        "synthetic_fixture: this output is not an empirical observation, calibrated model, or forecast",
        "contract_smoke: schema conformance does not establish scientific validation or source authenticity",
        "domain_csv_is_authoritative: outputs[] is a numeric index; the complete domain CSV is retained as an artifact",
    ]
    if replay_bytes is not None:
        replay_sha = sha256(replay_bytes)
        deterministic = declared_deterministic and replay_sha == csv_sha
        artifacts.append({
            "artifact_id": "civbench-replay-csv",
            "role": "same-seed-repeatability-comparison",
            "media_type": "text/csv",
            "artifact_ref": f"urn:sha256:{replay_sha}",
            "artifact_sha256": replay_sha,
            "json_pointer": None,
        })
        if deterministic:
            validation_evidence.append({
                "evidence_id": "byte-identical-domain-csv-replay",
                "kind": "reproducibility",
                "artifact_ref": f"urn:sha256:{replay_sha}",
                "artifact_sha256": replay_sha,
                "description": "A second output with the same scenario and seed is byte-identical to the primary domain CSV.",
            })
        elif declared_deterministic:
            warnings.append("repeatability_check_failed: second same-seed CSV differs byte-for-byte")
        else:
            warnings.append("engine_declares_nondeterministic: identical sample bytes do not establish a deterministic engine")
    elif declared_deterministic:
        warnings.append("determinism_unknown: engine declares deterministic but no second same-seed output was supplied")
    else:
        warnings.append("engine_declares_nondeterministic: determinism is false by scenario declaration")

    seed = engine.get("seed")
    replicates = engine.get("replicates")
    if not isinstance(seed, int) or seed < 0 or seed > 9_007_199_254_740_991:
        raise AdapterError("scenario engine seed is invalid")
    if not isinstance(replicates, int) or replicates < 1 or replicates > 1_000_000:
        raise AdapterError("scenario engine replicates is invalid")

    run_id = f"{scenario_id[:140]}.{track}.{csv_sha[:16]}"
    envelope = {
        "schema_version": RUN_SCHEMA_VERSION,
        "run_id": run_id,
        "scenario_id": scenario_id,
        "inputs": {
            "scenario_sha256": scenario_sha,
            "dataset_sha256": dataset_sha,
            "scenario_identity_kind": "scenario_document_bytes",
            "resolved_config_sha256": None,
            "resolved_params_sha256": None,
            "selected_definition_source": f"Symtropy CivBench {track} reference",
        },
        "engine": {
            "adapter": ADAPTER_ID,
            "engine_version": str(engine.get("engine_version", "unknown")),
            "model_sha256": model_sha,
            "model_identity_kind": "unknown",
            "source_revision": source_revision,
            "definition_source": f"Symtropy research/{track}",
        },
        "run_status": "completed",
        "output_classification": "synthetic_fixture",
        "deterministic": deterministic,
        "seed": seed,
        "replicates": replicates,
        "outputs": outputs,
        "validation": {
            "status": "contract_smoke",
            "notes": [
                "Domain report structure and exact input/output digests were checked.",
                "No empirical calibration, causal validation, prospective forecast, or external review is established by this adapter.",
            ],
            "evidence": validation_evidence,
        },
        "warnings": warnings,
        "artifacts": artifacts,
    }
    return envelope


def write_atomic(path: Path, raw: bytes, overwrite: bool) -> None:
    if len(raw) > MAX_OUTPUT_BYTES:
        raise AdapterError(f"run envelope exceeds the {MAX_OUTPUT_BYTES}-byte limit")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and not overwrite:
        raise AdapterError(f"output already exists: {path}")
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb", dir=path.parent, prefix=f".{path.name}.", suffix=".tmp", delete=False
        ) as stream:
            temporary = Path(stream.name)
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        if overwrite:
            os.replace(temporary, path)
            temporary = None
        else:
            os.link(temporary, path)
            temporary.unlink()
            temporary = None
    finally:
        if temporary is not None:
            try:
                temporary.unlink()
            except FileNotFoundError:
                pass


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--track", choices=("service-cascade", "knowledge-resilience"), required=True)
    parser.add_argument("--csv", type=Path, required=True)
    parser.add_argument("--scenario", type=Path, required=True)
    parser.add_argument("--dataset", type=Path, required=True)
    parser.add_argument("--model-source", type=Path, action="append", required=True)
    parser.add_argument("--source-revision", default=None)
    parser.add_argument("--replay-csv", type=Path, default=None)
    parser.add_argument("--csv-ref", default=None)
    parser.add_argument("--scenario-ref", default=None)
    parser.add_argument("--dataset-ref", default=None)
    parser.add_argument("--model-manifest-ref", default=None)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--overwrite", action="store_true")
    args = parser.parse_args(argv)

    try:
        csv_bytes = read_bounded(args.csv, MAX_SOURCE_BYTES)
        scenario_bytes = read_bounded(args.scenario, MAX_OUTPUT_BYTES)
        dataset_bytes = read_bounded(args.dataset, MAX_OUTPUT_BYTES)
        model_sha, model_manifest = _model_manifest(args.model_source)
        replay_bytes = read_bounded(args.replay_csv, MAX_SOURCE_BYTES) if args.replay_csv else None

        def ref(explicit: str | None, digest: str) -> str:
            return explicit if explicit else f"urn:sha256:{digest}"

        envelope = build_envelope(
            track=args.track,
            csv_bytes=csv_bytes,
            csv_ref=ref(args.csv_ref, sha256(csv_bytes)),
            scenario_bytes=scenario_bytes,
            scenario_ref=ref(args.scenario_ref, sha256(scenario_bytes)),
            dataset_bytes=dataset_bytes,
            dataset_ref=ref(args.dataset_ref, sha256(dataset_bytes)),
            model_sha=model_sha,
            model_manifest=model_manifest,
            model_manifest_ref=ref(args.model_manifest_ref, sha256(model_manifest)),
            source_revision=args.source_revision,
            replay_bytes=replay_bytes,
        )
        schema_path = Path(__file__).resolve().parent / "schemas" / "run-v0.1.schema.json"
        try:
            import jsonschema  # type: ignore
        except ImportError as exc:
            raise AdapterError("jsonschema==4.26.0 is required; schema validation cannot be skipped") from exc
        schema = strict_json(read_bounded(schema_path, MAX_OUTPUT_BYTES), "run-v0.1 schema")
        jsonschema.Draft202012Validator.check_schema(schema)
        try:
            jsonschema.Draft202012Validator(schema).validate(envelope)
        except jsonschema.ValidationError as exc:
            raise AdapterError(f"run-v0.1 schema validation failed: {exc.message}") from exc

        rendered = (json.dumps(envelope, ensure_ascii=False, allow_nan=False, indent=2) + "\n").encode("utf-8")
        write_atomic(args.output, rendered, args.overwrite)
        print(f"wrote {args.output} ({len(envelope['outputs'])} metric records)")
        return 0
    except (OSError, AdapterError, ValueError, RecursionError) as exc:
        print(f"CivBench adapter rejected input: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
