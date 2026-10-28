#!/usr/bin/env python3
"""Research-only adapter from CityLearn's Python simulator to Symtropy receipts.

This is a benchmark integration shim, not production control code. Controller
policies are chosen from the exact pinned CityLearn release; no predicted_*
observations are exposed to any baseline in this first panel.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib
import importlib.metadata
import json
import math
import platform
import resource
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

PROTOCOL_RELATIVE = Path(
    "tools/energy-dispatch-audit/fixtures/energy-citylearn-control-benchmark-v0.1.json"
)
EXPECTED_COMMIT = "834575c1a0194c8ae9d648ae858376a94dfceb78"
EXPECTED_DATASET_ID = "citylearn_challenge_2022_phase_all_plus_evs"
EXPECTED_SCHEMA_BLOB_SHA1 = "5d58d31fd75e4845d73d319c69df34d64223f533"
EXPECTED_CITYLEARN_VERSION = "3.0.2"
EXPECTED_STEP_COUNT = 8760
EXPECTED_BUILDINGS = 17
EXPECTED_FILE_COUNT = 30
REQUIRED_CONTROLLER_IDS = (
    "passive_no_control",
    "business_as_usual",
    "basic_rbc",
    "optimized_rbc",
)
REQUIRED_SPLITS = (
    ("train", 0, 5255, 5256),
    ("validation", 5256, 6131, 876),
    ("test", 6132, 8759, 2628),
)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")


def normalize_json(value: Any) -> Any:
    """Normalize NumPy/Pandas scalar values without allowing NaN/Infinity."""
    if value is None or isinstance(value, (str, bool, int)):
        return value
    if hasattr(value, "tolist"):
        try:
            value = value.tolist()
        except (ValueError, TypeError):
            pass
    elif hasattr(value, "item"):
        try:
            value = value.item()
        except (ValueError, TypeError):
            pass
    if value is None or isinstance(value, (str, bool, int)):
        return value
    if isinstance(value, float):
        return value if math.isfinite(value) else None
    if isinstance(value, dict):
        return {str(key): normalize_json(item) for key, item in value.items()}
    if isinstance(value, (tuple, list)):
        return [normalize_json(item) for item in value]
    if hasattr(value, "isoformat"):
        return value.isoformat()
    return str(value)


def validate_protocol(protocol: dict[str, Any]) -> None:
    """Fail closed on data pins, split coverage, or baseline registry drift."""
    if protocol.get("schema_version") != "energy-citylearn-control-benchmark-protocol-v0.1":
        raise ValueError("unsupported control benchmark protocol schema")
    if protocol.get("benchmark_id") != "energy-citylearn-pinned-baseline-panel-v0.1":
        raise ValueError("control benchmark identity changed")
    if "exploratory" not in protocol.get("claim_ceiling", "").lower():
        raise ValueError("exploratory-only claim ceiling is mandatory")

    source = protocol.get("source", {})
    source_expected = {
        "repository": "https://github.com/citylearn-project/CityLearn",
        "release_tag": "v3.0.2",
        "commit": EXPECTED_COMMIT,
        "dataset_id": EXPECTED_DATASET_ID,
        "schema_git_blob_sha1": EXPECTED_SCHEMA_BLOB_SHA1,
        "time_steps": EXPECTED_STEP_COUNT,
        "seconds_per_time_step": 3600,
        "buildings": EXPECTED_BUILDINGS,
        "random_seed": 2022,
        "raw_sha256_receipt_required": True,
        "dataset_terms_review_required_before_simulation": True,
        "source_dataset_csvs_must_not_be_committed_to_symtropy": True,
    }
    for key, expected in source_expected.items():
        if source.get(key) != expected:
            raise ValueError(f"source pin or claim contract changed: {key}")

    splits = protocol.get("splits", [])
    actual_splits = tuple(
        (entry.get("split_id"), entry.get("start_inclusive"), entry.get("end_inclusive"), entry.get("steps"))
        for entry in splits
    )
    if actual_splits != REQUIRED_SPLITS:
        raise ValueError("split order, bounds or row counts differ from frozen chronology")
    if splits[0]["start_inclusive"] != 0 or splits[-1]["end_inclusive"] != EXPECTED_STEP_COUNT - 1:
        raise ValueError("split partition does not cover the exact dataset window")
    for previous, current in zip(splits, splits[1:]):
        if previous["end_inclusive"] + 1 != current["start_inclusive"]:
            raise ValueError("split windows overlap or contain a gap")

    controllers = protocol.get("controllers", [])
    if tuple(entry.get("controller_id") for entry in controllers) != REQUIRED_CONTROLLER_IDS:
        raise ValueError("baseline controller order or IDs changed")
    if any(entry.get("learned") is not False or entry.get("uses_prediction_features") is not False for entry in controllers):
        raise ValueError("v0.1 is a fixed-baseline panel and must not enable learned/prediction-input policies")

    environment = protocol.get("environment", {})
    if (
        environment.get("interface") != "flat"
        or environment.get("central_agent") is not False
        or environment.get("offline") is not True
        or environment.get("disable_all_forecast_named_observations") is not True
        or environment.get("require_no_forbidden_observations_reach_policy") is not True
    ):
        raise ValueError("environment or forecast exclusion contract changed")

    if protocol.get("metrics", {}).get("native_kpi_method") != "CityLearnEnv.evaluate_v2(include_business_as_usual=False)":
        raise ValueError("native KPI method changed")
    if protocol.get("promotions", {}).get("source_terms_must_be_reviewed_before_control_run") is not True:
        raise ValueError("data rights review must remain a precondition")


def run_git(root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *args],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"git {args!r} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def validate_source_checkout(
    source_root: Path,
    dataset_root: Path,
    source_audit_path: Path,
    source_audit_receipts_path: Path,
    protocol: dict[str, Any],
) -> tuple[dict[str, Any], str, str]:
    source = protocol["source"]
    commit = run_git(source_root, "rev-parse", "HEAD")
    if commit != source["commit"]:
        raise ValueError(f"CityLearn commit mismatch: expected {source['commit']}, got {commit}")

    schema_path = Path(source["schema_path"])
    schema_full_path = source_root / schema_path
    if not schema_full_path.is_file():
        raise FileNotFoundError(f"pinned dataset schema missing: {schema_full_path}")
    schema_blob = run_git(source_root, "rev-parse", f"HEAD:{source['schema_path']}")
    if schema_blob != source["schema_git_blob_sha1"]:
        raise ValueError(f"schema Git blob mismatch: expected {source['schema_git_blob_sha1']}, got {schema_blob}")
    schema_bytes = schema_full_path.read_bytes()
    if len(schema_bytes) != source["expected_schema_bytes"]:
        raise ValueError("schema byte count differs from the frozen contract")
    schema = json.loads(schema_bytes)
    if (
        schema.get("start_date") != source["start_date"]
        or schema.get("simulation_start_time_step") != 0
        or schema.get("simulation_end_time_step") != EXPECTED_STEP_COUNT - 1
        or schema.get("seconds_per_time_step") != source["seconds_per_time_step"]
        or len(schema.get("buildings", {})) != source["buildings"]
    ):
        raise ValueError("actual schema coverage differs from protocol")
    if not dataset_root.is_dir() or dataset_root.resolve() != (source_root / source["dataset_path"]).resolve():
        raise ValueError("dataset root does not correspond to the frozen source path")

    if not source_audit_path.is_file() or not source_audit_receipts_path.is_file():
        raise FileNotFoundError("raw source audit report and receipts are required")
    source_audit_bytes = source_audit_path.read_bytes()
    source_audit = json.loads(source_audit_bytes)
    receipt_lines = [line for line in source_audit_receipts_path.read_text(encoding="utf-8").splitlines() if line.strip()]
    if (
        source_audit.get("status") != "PASS_PINNED_SOURCE_BYTES_ONLY"
        or source_audit.get("dataset_id") != source["dataset_id"]
        or source_audit.get("source_commit") != source["commit"]
        or source_audit.get("source_commit_verified") is not True
        or source_audit.get("schema_git_blob_identity_verified") is not True
        or source_audit.get("schema_csv_references_match_manifest") is not True
        or source_audit.get("source_file_git_blob_identities_verified") is not True
        or source_audit.get("raw_sha256_receipts_generated") is not True
        or source_audit.get("file_count") != EXPECTED_FILE_COUNT
        or source_audit.get("dataset_terms_cleared") is not False
        or source_audit.get("forecast_issue_times_verified") is not False
        or len(source_audit.get("files", [])) != EXPECTED_FILE_COUNT
        or len(receipt_lines) != EXPECTED_FILE_COUNT
    ):
        raise ValueError("source audit receipts are missing, stale or overstate qualification")

    audit_files = source_audit["files"]
    for record in audit_files:
        if len(record.get("sha256", "")) != 64 or any(c not in "0123456789abcdef" for c in record["sha256"]):
            raise ValueError(f"invalid raw SHA-256 receipt for {record.get('path')}")
    receipt_paths = {line.split("  ", 1)[1].split(" [git-blob=", 1)[0] for line in receipt_lines}
    report_paths = {record["path"] for record in audit_files}
    if receipt_paths != report_paths:
        raise ValueError("raw receipt text paths differ from source audit report paths")
    if source_audit.get("total_source_bytes_verified") != sum(record["size_bytes"] for record in audit_files):
        raise ValueError("source audit total byte count is internally inconsistent")

    return schema, commit, sha256_bytes(source_audit_bytes)


def percentile(values: list[float], percentile_value: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    index = max(0, min(len(ordered) - 1, math.ceil(percentile_value * len(ordered)) - 1))
    return ordered[index]


def build_agent(env: Any, dotted_path: str) -> Any:
    module_name, class_name = dotted_path.rsplit(".", 1)
    cls = getattr(importlib.import_module(module_name), class_name)
    return cls(env)


def run_episode(
    *,
    env: Any,
    controller: Any,
    controller_id: str,
    split_id: str,
    start: int,
    expected_steps: int,
    output_trace_path: Path,
) -> dict[str, Any]:
    observations, _ = env.reset(seed=2022)
    controller.reset()

    observation_names = list(getattr(controller, "observation_names", []))
    forbidden = [
        name
        for per_agent in observation_names
        for name in per_agent
        if "_predicted_" in name or name.endswith("_predicted")
    ]
    if forbidden:
        raise ValueError(f"forecast observations leaked to {controller_id}: {sorted(set(forbidden))}")

    latency_ms: list[float] = []
    trace_hasher = hashlib.sha256()
    steps = 0
    terminated = truncated = False
    simulation_started = time.perf_counter_ns()

    with output_trace_path.open("ab") as trace_file:
        while not (terminated or truncated):
            tick_start = time.perf_counter_ns()
            actions = controller.predict(observations, deterministic=True)
            tick_elapsed_ms = (time.perf_counter_ns() - tick_start) / 1_000_000.0
            latency_ms.append(tick_elapsed_ms)

            trace_record = {
                "controller_id": controller_id,
                "split_id": split_id,
                "global_time_index": start + steps,
                "actions": normalize_json(actions),
            }
            trace_line = canonical_json_bytes(trace_record) + b"\n"
            trace_file.write(trace_line)
            trace_hasher.update(trace_line)

            observations, _, terminated, truncated, _ = env.step(actions)
            steps += 1
            if steps > expected_steps:
                raise ValueError(f"{controller_id}/{split_id}: simulator exceeded expected time steps")

    simulation_elapsed_ms = (time.perf_counter_ns() - simulation_started) / 1_000_000.0
    if steps != expected_steps:
        raise ValueError(f"{controller_id}/{split_id}: expected {expected_steps} steps, got {steps}")

    native_kpis = env.evaluate_v2(include_business_as_usual=False)
    if not hasattr(native_kpis, "to_dict"):
        raise TypeError("CityLearn evaluate_v2 returned a non-tabular result")
    kpi_rows = [normalize_json(row) for row in native_kpis.to_dict(orient="records")]

    return {
        "controller_id": controller_id,
        "split_id": split_id,
        "global_start_inclusive": start,
        "steps_executed": steps,
        "forecast_observations_exposed": 0,
        "per_step_action_trace_sha256": trace_hasher.hexdigest(),
        "decision_latency_ms": {
            "p50": percentile(latency_ms, 0.50),
            "p95": percentile(latency_ms, 0.95),
            "p99": percentile(latency_ms, 0.99),
            "sample_count": len(latency_ms),
        },
        "simulation_loop_wall_ms": simulation_elapsed_ms,
        "native_kpi_rows": kpi_rows,
        "native_kpi_row_count": len(kpi_rows),
    }


def run_panel(args: argparse.Namespace) -> dict[str, Any]:
    if not args.acknowledge_data_terms_review or not args.data_terms_review_reference.strip():
        raise ValueError(
            "control execution is blocked until dataset data terms are reviewed; "
            "pass --acknowledge-data-terms-review and --data-terms-review-reference only after review"
        )

    source_root = args.source_checkout_root.resolve()
    dataset_root = args.dataset_root.resolve()
    protocol_bytes = args.protocol.read_bytes()
    protocol = json.loads(protocol_bytes)
    validate_protocol(protocol)
    schema, commit, source_audit_sha256 = validate_source_checkout(
        source_root,
        dataset_root,
        args.source_audit_report.resolve(),
        args.source_audit_receipts.resolve(),
        protocol,
    )

    installed_version = importlib.metadata.version("citylearn")
    if installed_version != EXPECTED_CITYLEARN_VERSION:
        raise ValueError(f"CityLearn package version mismatch: expected {EXPECTED_CITYLEARN_VERSION}, got {installed_version}")

    forecast_names = sorted(
        name for name in schema["observations"]
        if "_predicted_" in name or name.endswith("_predicted")
    )
    if not forecast_names:
        raise ValueError("pinned schema did not expose forecast-name registry; refusing silent contract drift")

    # Import only after the protocol, rights acknowledgment and source receipts pass.
    from citylearn.citylearn import CityLearnEnv

    output_trace = args.output_dir / "action-trace.jsonl"
    args.output_dir.mkdir(parents=True, exist_ok=True)
    if output_trace.exists():
        output_trace.unlink()

    results = []
    wall_start = time.perf_counter_ns()
    for split in protocol["splits"]:
        for spec in protocol["controllers"]:
            env = CityLearnEnv(
                schema=str(source_root / protocol["source"]["schema_path"]),
                root_directory=str(dataset_root),
                simulation_start_time_step=split["start_inclusive"],
                simulation_end_time_step=split["end_inclusive"],
                episode_time_steps=split["steps"],
                central_agent=False,
                interface="flat",
                random_seed=protocol["source"]["random_seed"],
                offline=True,
                inactive_observations=forecast_names,
                render_mode="none",
                export_kpis_on_episode_end=False,
            )
            try:
                controller = build_agent(env, spec["python_class"])
                results.append(run_episode(
                    env=env,
                    controller=controller,
                    controller_id=spec["controller_id"],
                    split_id=split["split_id"],
                    start=split["start_inclusive"],
                    expected_steps=split["steps"],
                    output_trace_path=output_trace,
                ))
            finally:
                env.close()

    elapsed_wall_ms = (time.perf_counter_ns() - wall_start) / 1_000_000.0
    expected_rows = len(protocol["splits"]) * len(protocol["controllers"])
    if len(results) != expected_rows or any(result["forecast_observations_exposed"] != 0 for result in results):
        raise ValueError("baseline panel coverage or forecast exclusion is incomplete")

    peak_rss_kib = int(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss)
    trace_bytes = output_trace.read_bytes()
    receipt_payload = {
        "schema_version": "energy-citylearn-control-baseline-report-v0.1",
        "benchmark_id": protocol["benchmark_id"],
        "status": "PASS_SIMULATED_BASELINE_PANEL_ONLY",
        "claim_ceiling": protocol["claim_ceiling"],
        "source": {
            "dataset_id": protocol["source"]["dataset_id"],
            "release_tag": protocol["source"]["release_tag"],
            "source_commit": commit,
            "schema_git_blob_sha1": protocol["source"]["schema_git_blob_sha1"],
            "source_audit_report_sha256": source_audit_sha256,
            "source_raw_sha256_receipts_verified": True,
            "dataset_terms_review_acknowledged": True,
            "dataset_terms_review_reference": args.data_terms_review_reference.strip(),
            "forecast_issue_times_verified": False,
            "forecast_observations_exposed": 0,
        },
        "runtime": {
            "python_version": sys.version.split()[0],
            "platform": platform.platform(),
            "citylearn_package_version": installed_version,
            "peak_process_rss_kib": peak_rss_kib,
            "benchmark_wall_ms": elapsed_wall_ms,
            "seed": protocol["source"]["random_seed"],
        },
        "protocol_sha256": sha256_bytes(protocol_bytes),
        "action_trace_sha256": sha256_bytes(trace_bytes),
        "action_trace_bytes": len(trace_bytes),
        "window_state_policy": protocol["environment"]["initial_state_policy"],
        "results": results,
        "comparison_rule": "Do not collapse KPI rows into a sole scalar. Compare matched controller/window pairs and preserve all native KPI rows; this panel is not a SOTA claim.",
    }
    receipt_payload_sha256 = sha256_bytes(canonical_json_bytes(receipt_payload))
    wrapper = {
        "report": receipt_payload,
        "report_payload_sha256": receipt_payload_sha256,
    }
    output_path = args.output_dir / "report.json"
    output_path.write_text(json.dumps(wrapper, indent=2, sort_keys=True, ensure_ascii=False, allow_nan=False) + "\n", encoding="utf-8")
    return {
        "output": str(output_path),
        "report_payload_sha256": receipt_payload_sha256,
        "results": len(results),
        "action_trace_sha256": sha256_bytes(trace_bytes),
        "benchmark_wall_ms": elapsed_wall_ms,
        "peak_process_rss_kib": peak_rss_kib,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-checkout-root", type=Path, required=True)
    parser.add_argument("--dataset-root", type=Path, required=True)
    parser.add_argument("--source-audit-report", type=Path, required=True)
    parser.add_argument("--source-audit-receipts", type=Path, required=True)
    parser.add_argument("--protocol", type=Path, default=PROTOCOL_RELATIVE)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--acknowledge-data-terms-review", action="store_true")
    parser.add_argument("--data-terms-review-reference", default="")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        result = run_panel(args)
    except Exception as exc:  # noqa: BLE001 - CI entry point must fail closed and explain.
        print(f"FAIL CityLearn baseline panel: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2
    print("PASS CityLearn simulated baseline panel only")
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
