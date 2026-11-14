#!/usr/bin/env python3
"""Plan and validate auditable HarmonyField factorial experiments.

This tool creates run plans and checks result manifests; it does NOT execute
Symtropy simulations or estimate scientific utility. Python 3.10+, stdlib only.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import sys
import tempfile
from pathlib import Path

PLAN_SCHEMA = "luminous.harmony-field-ablation-plan.v1"
RESULT_SCHEMA = "luminous.harmony-field-ablation-result.v1"
PROTOCOL_VERSION = "0.1"
SHA_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
STATUSES = {"completed", "failed", "skipped", "cancelled", "unqualified"}
REPLAY = {"exact_match", "bounded_nondeterminism_accepted", "not_tested", "mismatch"}


class ValidationError(ValueError):
    pass


def canonical(value):
    """Stable canonicalization used for this tool's SHA-256 receipts."""
    try:
        return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False,
                          allow_nan=False).encode("utf-8")
    except (TypeError, ValueError) as exc:
        raise ValidationError(f"not canonical-JSON serializable: {exc}") from exc


def digest(value):
    return hashlib.sha256(canonical(value)).hexdigest()


def is_sha(value):
    return isinstance(value, str) and bool(SHA_RE.fullmatch(value))


def file_sha256(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def artifact_sha256(root, relative_name):
    """Hash an evidence file confined to the declared artifact root."""
    if not isinstance(relative_name, str) or not relative_name.strip():
        raise ValidationError("artifact path must be a non-empty relative path")
    relative = Path(relative_name)
    if relative.is_absolute() or ".." in relative.parts:
        raise ValidationError("artifact path must be relative and may not traverse parents")
    root = root.resolve()
    try:
        target = (root / relative).resolve(strict=True)
    except OSError as exc:
        raise ValidationError(f"artifact file is missing or unreadable: {relative_name}") from exc
    if target == root or root not in target.parents:
        raise ValidationError("artifact path resolves outside the declared artifact root")
    if not target.is_file():
        raise ValidationError(f"artifact path is not a regular file: {relative_name}")
    try:
        return file_sha256(target)
    except OSError as exc:
        raise ValidationError(f"cannot hash artifact {relative_name}: {exc}") from exc


def arms():
    """A000..A111, with F/R/E stored in bits 2/1/0 respectively."""
    return [{
        "arm_id": f"A{(i >> 2) & 1}{(i >> 1) & 1}{i & 1}",
        "arm_index": i,
        "friction_modulation": bool(i & 4),
        "resonance_prediction_error": bool(i & 2),
        "emotional_contagion": bool(i & 1),
    } for i in range(8)]


def parse_list(raw, cast, label, predicate, unique=True):
    try:
        values = [cast(part.strip()) for part in raw.split(",") if part.strip()]
    except ValueError as exc:
        raise argparse.ArgumentTypeError(f"{label}: {exc}") from exc
    if not values or any(not predicate(v) for v in values):
        raise argparse.ArgumentTypeError(f"invalid {label} values")
    if unique and len(values) != len(set(values)):
        raise argparse.ArgumentTypeError(f"{label} contains duplicates")
    return values


def finite_number(value):
    return (not isinstance(value, bool) and isinstance(value, (int, float))
            and math.isfinite(value))


def read_object(path):
    try:
        obj = json.loads(path.read_text(encoding="utf-8"))
        canonical(obj)
    except (OSError, json.JSONDecodeError, ValidationError) as exc:
        raise ValidationError(f"cannot read canonical JSON object {path}: {exc}") from exc
    if not isinstance(obj, dict):
        raise ValidationError(f"{path} must contain a JSON object")
    return obj


def validate_scenario(scenario):
    endpoint = scenario.get("primary_endpoint")
    if not isinstance(endpoint, dict):
        raise ValidationError("scenario needs a preregistered primary_endpoint object")
    for key in ("name", "units"):
        if not isinstance(endpoint.get(key), str) or not endpoint[key].strip():
            raise ValidationError(f"primary_endpoint.{key} must be a non-empty string")
    direction = endpoint.get("direction")
    if not isinstance(direction, str) or direction not in {"higher_is_better", "lower_is_better"}:
        raise ValidationError("primary_endpoint.direction must be higher_is_better or lower_is_better")
    threshold = endpoint.get("meaningful_effect_threshold")
    if not finite_number(threshold) or threshold < 0:
        raise ValidationError("primary_endpoint.meaningful_effect_threshold must be finite and non-negative")
    if not isinstance(scenario.get("task_description"), str) or not scenario["task_description"].strip():
        raise ValidationError("scenario needs a non-empty task_description")
    solver = scenario.get("solver_and_timestep")
    if not isinstance(solver, dict) or not solver:
        raise ValidationError("scenario needs a non-empty solver_and_timestep object")

    random_contract = scenario.get("random_stream_contract")
    if not isinstance(random_contract, dict):
        raise ValidationError("scenario needs a random_stream_contract object")
    strategy = random_contract.get("strategy")
    strategies = {
        "counter_based_event_keyed",
        "independent_component_substreams",
        "single_stateful_stream",
        "deterministic_no_randomness",
    }
    if not isinstance(strategy, str) or strategy not in strategies:
        raise ValidationError("random_stream_contract.strategy must identify the actual RNG strategy")
    alignment = random_contract.get("treatment_alignment")
    alignments = {
        "event_keyed",
        "common_random_numbers_verified",
        "same_seed_only",
        "not_applicable",
    }
    if not isinstance(alignment, str) or alignment not in alignments:
        raise ValidationError("random_stream_contract.treatment_alignment must explicitly identify cross-treatment alignment")
    evidence = random_contract.get("alignment_evidence")
    if not isinstance(evidence, str) or not evidence.strip():
        raise ValidationError("random_stream_contract.alignment_evidence must explain how alignment was implemented or why it is unavailable")
    evidence_path = random_contract.get("alignment_evidence_path")
    if not isinstance(evidence_path, str) or not evidence_path.strip():
        raise ValidationError("random_stream_contract.alignment_evidence_path must name a verifiable evidence file")
    evidence_relative = Path(evidence_path)
    if evidence_relative.is_absolute() or ".." in evidence_relative.parts:
        raise ValidationError("random_stream_contract.alignment_evidence_path must be relative and must not traverse parents")
    if not is_sha(random_contract.get("alignment_evidence_sha256")):
        raise ValidationError("random_stream_contract.alignment_evidence_sha256 must be a lowercase SHA-256 digest")
    if strategy == "counter_based_event_keyed" and alignment != "event_keyed":
        raise ValidationError("counter-based event-keyed RNG requires treatment_alignment=event_keyed")
    if strategy == "deterministic_no_randomness" and alignment != "not_applicable":
        raise ValidationError("deterministic-no-randomness runs require treatment_alignment=not_applicable")
    if alignment == "event_keyed" and strategy != "counter_based_event_keyed":
        raise ValidationError("event_keyed alignment requires counter_based_event_keyed strategy")


def make_plan(args):
    commit = args.candidate_commit.lower()
    if not COMMIT_RE.fullmatch(commit):
        raise ValidationError("--candidate-commit must be the full 40-character Git SHA")
    if not args.task_id.strip():
        raise ValidationError("--task-id must not be empty")
    seeds, held = args.seeds, args.held_out_seeds
    if not set(held).issubset(seeds):
        raise ValidationError("every held-out seed must also appear in --seeds")
    development = [s for s in seeds if s not in set(held)]
    if not development:
        raise ValidationError("at least one development seed must remain")
    scenario = read_object(args.scenario_config)
    validate_scenario(scenario)
    scenario_sha = digest(scenario)
    tool_sha = file_sha256(Path(__file__))
    arm_list = arms()
    runs = []
    for seed in seeds:
        partition = "held_out" if seed in set(held) else "development"
        for dim in args.dimensions:
            for strength in args.source_strengths:
                for radius in args.radii:
                    for arm in arm_list:
                        config = {
                            "protocol_version": PROTOCOL_VERSION,
                            "candidate_commit": commit,
                            "task_id": args.task_id.strip(),
                            "scenario_config_sha256": scenario_sha,
                            "seed": seed,
                            "seed_partition": partition,
                            "dimension": dim,
                            "source_strength": strength,
                            "influence_radius": radius,
                            "arm_id": arm["arm_id"],
                            "arm_index": arm["arm_index"],
                            "mechanisms": {
                                "friction_modulation": arm["friction_modulation"],
                                "resonance_prediction_error": arm["resonance_prediction_error"],
                                "emotional_contagion": arm["emotional_contagion"],
                            },
                        }
                        cfg_sha = digest(config)
                        runs.append({
                            "run_id": "run-" + cfg_sha,
                            "configuration_sha256": cfg_sha,
                            "configuration": config,
                            "run_status": "planned",
                        })
    unsigned = {
        "schema_version": PLAN_SCHEMA,
        "protocol_version": PROTOCOL_VERSION,
        "tool_sha256": tool_sha,
        "candidate_commit": commit,
        "task_id": args.task_id.strip(),
        "scenario_config": scenario,
        "scenario_config_sha256": scenario_sha,
        "seed_partition": {"development": development, "held_out": held},
        "factorial_arms": arm_list,
        "parameter_grid": {
            "dimensions": args.dimensions,
            "source_strengths": args.source_strengths,
            "influence_radii": args.radii,
            "kernel_exponent_rule": "max(dimension - 1, 1)",
            "kernel_exponent_is_runtime_parameter": False,
        },
        "run_count": len(runs),
        "planned_runs": runs,
        "plan_status": "planned_not_executed",
    }
    return {**unsigned, "plan_sha256": digest(unsigned)}


def validate_plan(plan):
    errors = []
    if not isinstance(plan, dict) or plan.get("schema_version") != PLAN_SCHEMA:
        return ["unsupported plan schema"]
    if not is_sha(plan.get("tool_sha256")):
        errors.append("tool_sha256 missing or malformed")
    unsigned = {k: v for k, v in plan.items() if k != "plan_sha256"}
    try:
        if plan.get("plan_sha256") != digest(unsigned):
            errors.append("plan_sha256 mismatch")
    except ValidationError as exc:
        errors.append(str(exc))
    commit, task = plan.get("candidate_commit"), plan.get("task_id")
    if not isinstance(commit, str) or not COMMIT_RE.fullmatch(commit):
        errors.append("candidate_commit must be a full lowercase Git SHA")
    if not isinstance(task, str) or not task.strip():
        errors.append("task_id missing")
    scenario = plan.get("scenario_config")
    try:
        if not isinstance(scenario, dict):
            raise ValidationError("must be an object")
        validate_scenario(scenario)
        scenario_sha = digest(scenario)
    except ValidationError as exc:
        errors.append(f"invalid scenario_config: {exc}")
        scenario_sha = None
    if plan.get("scenario_config_sha256") != scenario_sha:
        errors.append("scenario_config_sha256 mismatch")
    expected_arms = arms()
    if plan.get("factorial_arms") != expected_arms:
        errors.append("factorial arms do not equal canonical A000..A111 enumeration")
    partition, grid = plan.get("seed_partition"), plan.get("parameter_grid")
    runs = plan.get("planned_runs")
    if not isinstance(partition, dict) or not isinstance(grid, dict) or not isinstance(runs, list):
        return errors + ["seed_partition, parameter_grid, and planned_runs must have expected types"]
    dev, held = partition.get("development"), partition.get("held_out")
    dims, strengths, radii = grid.get("dimensions"), grid.get("source_strengths"), grid.get("influence_radii")
    seed_lists_ok = (isinstance(dev, list) and isinstance(held, list) and bool(dev) and bool(held)
        and all(isinstance(x, int) and not isinstance(x, bool) and x >= 0 for x in dev + held)
        and len(dev) == len(set(dev)) and len(held) == len(set(held)) and not set(dev) & set(held))
    grid_ok = (isinstance(dims, list) and bool(dims) and all(isinstance(x, int) and not isinstance(x, bool) and x > 0 for x in dims)
        and len(dims) == len(set(dims))
        and isinstance(strengths, list) and bool(strengths) and all(finite_number(x) and x >= 0 for x in strengths)
        and len(strengths) == len(set(strengths))
        and isinstance(radii, list) and bool(radii) and all(finite_number(x) and x > 0 for x in radii)
        and len(radii) == len(set(radii)))
    if not seed_lists_ok:
        errors.append("seed partitions must be non-empty, unique, disjoint non-negative integer lists")
    if not grid_ok:
        errors.append("parameter grid values must be non-empty, unique, and within finite valid ranges")
    if plan.get("run_count") != len(runs):
        errors.append("run_count mismatch")
    observed, seen_ids = set(), set()
    for i, rec in enumerate(runs):
        label = f"planned_runs[{i}]"
        if not isinstance(rec, dict) or not isinstance(rec.get("configuration"), dict):
            errors.append(f"{label} must contain a configuration object")
            continue
        cfg = rec["configuration"]
        try:
            cfg_sha = digest(cfg)
        except ValidationError as exc:
            errors.append(f"{label}: invalid config: {exc}")
            continue
        if rec.get("configuration_sha256") != cfg_sha:
            errors.append(f"{label}: configuration digest mismatch")
        run_id = rec.get("run_id")
        if not isinstance(run_id, str) or run_id != "run-" + cfg_sha:
            errors.append(f"{label}: run_id mismatch")
        if isinstance(run_id, str) and run_id in seen_ids:
            errors.append(f"{label}: duplicate run_id")
        if isinstance(run_id, str):
            seen_ids.add(run_id)
        observed.add(cfg_sha)
        if cfg.get("candidate_commit") != commit or cfg.get("task_id") != task:
            errors.append(f"{label}: candidate/task identity mismatch")
        if cfg.get("scenario_config_sha256") != scenario_sha:
            errors.append(f"{label}: scenario configuration mismatch")
    if seed_lists_ok and grid_ok and scenario_sha and isinstance(commit, str) and isinstance(task, str):
        expected = set()
        for seed, group in [(s, "development") for s in dev] + [(s, "held_out") for s in held]:
            for dim in dims:
                for strength in strengths:
                    for radius in radii:
                        for arm in expected_arms:
                            cfg = {
                                "protocol_version": PROTOCOL_VERSION,
                                "candidate_commit": commit,
                                "task_id": task,
                                "scenario_config_sha256": scenario_sha,
                                "seed": seed,
                                "seed_partition": group,
                                "dimension": dim,
                                "source_strength": strength,
                                "influence_radius": radius,
                                "arm_id": arm["arm_id"],
                                "arm_index": arm["arm_index"],
                                "mechanisms": {
                                    "friction_modulation": arm["friction_modulation"],
                                    "resonance_prediction_error": arm["resonance_prediction_error"],
                                    "emotional_contagion": arm["emotional_contagion"],
                                },
                            }
                            expected.add(digest(cfg))
        if expected != observed:
            errors.append(f"factorial/grid coverage mismatch: {len(expected-observed)} missing, {len(observed-expected)} unexpected")
    return errors


def validate_result(record, planned, commit, artifact_root, plan_sha):
    errors = []
    label = "result " + planned["run_id"]
    if not isinstance(record, dict):
        return [label + " must be an object"]
    if record.get("schema_version") != RESULT_SCHEMA:
        errors.append(label + ": schema_version mismatch")
    if record.get("run_id") != planned["run_id"]:
        errors.append(label + ": run_id mismatch")
    if record.get("candidate_commit") != commit:
        errors.append(label + ": candidate_commit mismatch")
    if record.get("plan_sha256") != plan_sha:
        errors.append(label + ": plan_sha256 mismatch")
    if record.get("configuration_sha256") != planned["configuration_sha256"]:
        errors.append(label + ": configuration_sha256 mismatch")
    status = record.get("run_status")
    if not isinstance(status, str) or status not in STATUSES:
        errors.append(label + ": invalid run_status")
    for key in ("validation_failure_count", "nonfinite_count"):
        val = record.get(key)
        if not isinstance(val, int) or isinstance(val, bool) or val < 0:
            errors.append(label + f": {key} must be a non-negative integer")
    if not isinstance(record.get("termination_reason"), str):
        errors.append(label + ": termination_reason missing")
    if status == "completed":
        if not finite_number(record.get("primary_endpoint")):
            errors.append(label + ": completed run needs finite primary_endpoint")
        if not finite_number(record.get("invariant_drift")):
            errors.append(label + ": completed run needs finite invariant_drift")
        metrics = record.get("secondary_metrics")
        if not isinstance(metrics, dict) or any(not finite_number(v) for v in metrics.values()):
            errors.append(label + ": secondary_metrics must be an object of finite numbers")
        try:
            actual = artifact_sha256(artifact_root, record.get("artifact_path"))
            if record.get("artifact_sha256") != actual:
                errors.append(label + ": artifact_sha256 does not match artifact bytes")
        except ValidationError as exc:
            errors.append(label + f": artifact verification failed: {exc}")
    elif record.get("primary_endpoint") is not None:
        errors.append(label + ": non-completed run must have null primary_endpoint")
    replay = record.get("replay_status")
    if not isinstance(replay, str) or replay not in REPLAY:
        errors.append(label + ": invalid/missing replay_status")
    if isinstance(replay, str) and replay in {"exact_match", "bounded_nondeterminism_accepted"}:
        try:
            actual_replay = artifact_sha256(artifact_root, record.get("replay_evidence_path"))
            if record.get("replay_evidence_sha256") != actual_replay:
                errors.append(label + ": replay_evidence_sha256 does not match evidence bytes")
        except ValidationError as exc:
            errors.append(label + f": replay-evidence verification failed: {exc}")
    return errors


def atomic_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False, allow_nan=False) + "\n"
    fd, temp = tempfile.mkstemp(prefix="." + path.name + ".", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(text)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temp, path)
    finally:
        if os.path.exists(temp):
            os.unlink(temp)


def validate_results(plan, path, artifact_root=None):
    errors = validate_plan(plan)
    if errors:
        return {"validation_status": "invalid_plan", "errors": errors}
    records = []
    root = Path(artifact_root) if artifact_root is not None else path.parent
    rng_contract = plan["scenario_config"]["random_stream_contract"]
    try:
        actual_rng_evidence = artifact_sha256(root, rng_contract["alignment_evidence_path"])
        if actual_rng_evidence != rng_contract["alignment_evidence_sha256"]:
            errors.append("random-stream alignment evidence hash does not match artifact bytes")
    except ValidationError as exc:
        errors.append(f"random-stream alignment evidence verification failed: {exc}")
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        return {"validation_status": "invalid_results_file", "errors": [str(exc)]}
    for lineno, line in enumerate(lines, 1):
        if not line.strip():
            continue
        try:
            item = json.loads(line)
            canonical(item)
        except (json.JSONDecodeError, ValidationError) as exc:
            errors.append(f"line {lineno}: invalid JSON: {exc}")
            continue
        if not isinstance(item, dict):
            errors.append(f"line {lineno}: expected a JSON object")
            continue
        records.append(item)
    by_id, planned_by_id = {}, {x["run_id"]: x for x in plan["planned_runs"]}
    for item in records:
        run_id = item.get("run_id")
        if not isinstance(run_id, str) or run_id not in planned_by_id:
            errors.append(f"unexpected run_id: {run_id!r}")
            continue
        if run_id in by_id:
            errors.append(f"duplicate result for {run_id}")
            continue
        by_id[run_id] = item
        errors.extend(validate_result(item, planned_by_id[run_id], plan["candidate_commit"], root, plan["plan_sha256"]))
    missing = set(planned_by_id) - set(by_id)
    if missing:
        errors.append(f"{len(missing)} planned result record(s) missing; missing is not zero or pass")
    completed = sum(r.get("run_status") == "completed" for r in by_id.values())
    failed = sum(isinstance(r.get("run_status"), str) and r.get("run_status") in {"failed", "skipped", "cancelled", "unqualified"} for r in by_id.values())
    invalid_counters = sum(
        r.get("run_status") == "completed" and
        ((isinstance(r.get("validation_failure_count"), int) and not isinstance(r.get("validation_failure_count"), bool) and r["validation_failure_count"] > 0)
         or (isinstance(r.get("nonfinite_count"), int) and not isinstance(r.get("nonfinite_count"), bool) and r["nonfinite_count"] > 0))
        for r in by_id.values()
    )
    unreplayed = sum(not isinstance(r.get("replay_status"), str) or r.get("replay_status") not in {"exact_match", "bounded_nondeterminism_accepted"} for r in by_id.values())
    qualified = (not errors and len(by_id) == len(planned_by_id) and completed == len(planned_by_id)
                 and failed == 0 and invalid_counters == 0 and unreplayed == 0)
    return {
        "schema_version": "luminous.harmony-field-ablation-validation-report.v1",
        "plan_sha256": plan["plan_sha256"],
        "candidate_commit": plan["candidate_commit"],
        "task_id": plan["task_id"],
        "validator_tool_sha256": file_sha256(Path(__file__)),
        "python_version": sys.version.split()[0],
        "planned_run_count": len(planned_by_id),
        "result_record_count": len(records),
        "completed_run_count": completed,
        "failed_or_unqualified_run_count": failed,
        "completed_runs_with_invalid_counters": invalid_counters,
        "runs_without_accepted_replay_evidence": unreplayed,
        "missing_run_count": len(missing),
        "validation_status": "qualified_manifest" if qualified else "NOT_QUALIFIED",
        "errors": errors,
        "note": "Manifest qualification is not evidence of task utility or empirical/physical validity; no effect estimates are computed.",
    }


def percentile(values, probability):
    ordered = sorted(values)
    if not ordered:
        raise ValidationError("cannot calculate percentile from an empty sample")
    position = (len(ordered) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    fraction = position - lower
    return ordered[lower] * (1.0 - fraction) + ordered[upper] * fraction


def sample_variance(values):
    """Unbiased sample variance; returns None when fewer than two observations exist."""
    if len(values) < 2:
        return None
    mean = sum(values) / len(values)
    return sum((value - mean) ** 2 for value in values) / (len(values) - 1)


def bootstrap_mean_interval(values, replicates, rng):
    if not values:
        raise ValidationError("cannot estimate effect without paired seed-level values")
    if replicates < 100:
        raise ValidationError("bootstrap replicates must be at least 100")
    estimate = sum(values) / len(values)
    means = []
    for _ in range(replicates):
        sample = [values[rng.randrange(len(values))] for _ in values]
        means.append(sum(sample) / len(sample))
    return estimate, percentile(means, 0.025), percentile(means, 0.975)


def interpret_effect(lower, upper, threshold):
    if lower > threshold:
        return "meaningful_benefit_supported_by_interval"
    if upper < -threshold:
        return "meaningful_harm_supported_by_interval"
    return "inconclusive_relative_to_preregistered_threshold"


def analyze_results(plan, results_path, artifact_root=None, bootstrap_replicates=5000, bootstrap_seed=20261013):
    """Estimate held-out paired arm effects and factorial contrasts; no p-values or causal overclaim."""
    if not isinstance(bootstrap_replicates, int) or isinstance(bootstrap_replicates, bool) or bootstrap_replicates < 100:
        return {
            "schema_version": "luminous.harmony-field-ablation-analysis.v1",
            "analysis_status": "NOT_QUALIFIED",
            "errors": ["bootstrap_replicates must be an integer >= 100"],
        }
    validity = validate_results(plan, results_path, artifact_root)
    if validity.get("validation_status") != "qualified_manifest":
        return {
            "schema_version": "luminous.harmony-field-ablation-analysis.v1",
            "analysis_status": "NOT_QUALIFIED",
            "manifest_validation": validity,
            "errors": ["analysis refused because the result manifest is not mechanically qualified"],
        }
    held_out = plan["seed_partition"]["held_out"]
    if len(held_out) < 2:
        return {
            "schema_version": "luminous.harmony-field-ablation-analysis.v1",
            "analysis_status": "NOT_QUALIFIED",
            "manifest_validation": validity,
            "errors": ["at least two held-out seed blocks are required for the bootstrap interval"],
        }
    try:
        endpoint = plan["scenario_config"]["primary_endpoint"]
        threshold = float(endpoint["meaningful_effect_threshold"])
        sign = 1.0 if endpoint["direction"] == "higher_is_better" else -1.0
        raw_records = [json.loads(line) for line in results_path.read_text(encoding="utf-8").splitlines() if line.strip()]
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
        return {
            "schema_version": "luminous.harmony-field-ablation-analysis.v1",
            "analysis_status": "NOT_QUALIFIED",
            "manifest_validation": validity,
            "errors": [f"could not load validated analysis inputs: {exc}"],
        }
    records = {item["run_id"]: item for item in raw_records}
    grouped = {}
    for seed in held_out:
        for planned in plan["planned_runs"]:
            cfg = planned["configuration"]
            if cfg["seed"] != seed:
                continue
            group = (cfg["dimension"], cfg["source_strength"], cfg["influence_radius"])
            grouped.setdefault(group, {}).setdefault(seed, {})[cfg["arm_id"]] = (
                float(records[planned["run_id"]]["primary_endpoint"]) * sign
            )
    contrast_factors = [
        ("F", ("friction_modulation",)),
        ("R", ("resonance_prediction_error",)),
        ("E", ("emotional_contagion",)),
        ("F×R", ("friction_modulation", "resonance_prediction_error")),
        ("F×E", ("friction_modulation", "emotional_contagion")),
        ("R×E", ("resonance_prediction_error", "emotional_contagion")),
        ("F×R×E", ("friction_modulation", "resonance_prediction_error", "emotional_contagion")),
    ]
    arms_by_id = {a["arm_id"]: a for a in arms()}
    rng = __import__("random").Random(bootstrap_seed)
    variants = []
    for group in sorted(grouped):
        seed_values = grouped[group]
        missing_cells = [
            f"seed={seed},arm={arm['arm_id']}"
            for seed in held_out for arm in arms()
            if arm["arm_id"] not in seed_values.get(seed, {})
        ]
        if missing_cells:
            return {
                "schema_version": "luminous.harmony-field-ablation-analysis.v1",
                "analysis_status": "NOT_QUALIFIED",
                "manifest_validation": validity,
                "errors": [f"held-out factorial cells missing: {missing_cells[:10]}"],
            }
        seed_order = [seed for seed in held_out]
        arm_effects = []
        baseline_values = [seed_values[seed]["A000"] for seed in seed_order]
        for arm in arms():
            arm_id = arm["arm_id"]
            treatment_values = [seed_values[seed][arm_id] for seed in seed_order]
            deltas = [treatment - baseline for treatment, baseline in zip(treatment_values, baseline_values)]
            estimate, lower, upper = bootstrap_mean_interval(deltas, bootstrap_replicates, rng)
            if arm_id == "A000":
                pairing_diagnostic = {
                    "status": "not_applicable_baseline",
                    "paired_difference_sample_variance": 0.0,
                    "independent_difference_variance_estimate": None,
                    "paired_to_independent_variance_ratio": None,
                }
            else:
                paired_variance = sample_variance(deltas)
                independent_variance = sample_variance(treatment_values) + sample_variance(baseline_values)
                ratio = paired_variance / independent_variance if independent_variance and independent_variance > 0.0 else None
                if ratio is None:
                    status = "undefined_zero_or_unestimable_denominator"
                elif ratio < 1.0:
                    status = "paired_variance_lower_than_independent_estimate"
                elif ratio > 1.0:
                    status = "paired_variance_higher_than_independent_estimate"
                else:
                    status = "paired_variance_equal_to_independent_estimate"
                pairing_diagnostic = {
                    "status": status,
                    "paired_difference_sample_variance": paired_variance,
                    "independent_difference_variance_estimate": independent_variance,
                    "paired_to_independent_variance_ratio": ratio,
                }
            arm_effects.append({
                "arm_id": arm_id,
                "arm_index": arm["arm_index"],
                "paired_difference_by_held_out_seed": [
                    {"seed": seed, "difference_positive_is_better": delta}
                    for seed, delta in zip(seed_order, deltas)
                ],
                "mean_paired_difference_positive_is_better": estimate,
                "bootstrap_95_percentile_interval": [lower, upper],
                "pairing_variance_diagnostic": pairing_diagnostic,
                "interpretation": "baseline" if arm_id == "A000" else interpret_effect(lower, upper, threshold),
            })
        factorial_effects = []
        for name, keys in contrast_factors:
            per_seed = []
            for seed in seed_order:
                positive, negative = [], []
                for arm_id, values in seed_values[seed].items():
                    arm = arms_by_id[arm_id]
                    product = 1
                    for key in keys:
                        product *= 1 if arm[key] else -1
                    (positive if product == 1 else negative).append(values)
                per_seed.append(sum(positive) / len(positive) - sum(negative) / len(negative))
            estimate, lower, upper = bootstrap_mean_interval(per_seed, bootstrap_replicates, rng)
            factorial_effects.append({
                "contrast": name,
                "effect_positive_is_better": estimate,
                "bootstrap_95_percentile_interval": [lower, upper],
                "effect_by_held_out_seed": [
                    {"seed": seed, "effect_positive_is_better": value}
                    for seed, value in zip(seed_order, per_seed)
                ],
                "interpretation": interpret_effect(lower, upper, threshold),
            })
        variants.append({
            "dimension": group[0],
            "source_strength": group[1],
            "influence_radius": group[2],
            "held_out_seed_count": len(seed_order),
            "arm_vs_A000": arm_effects,
            "factorial_main_and_interaction_effects": factorial_effects,
        })
    return {
        "schema_version": "luminous.harmony-field-ablation-analysis.v1",
        "analysis_status": "analyzed_held_out_runs",
        "plan_sha256": plan["plan_sha256"],
        "candidate_commit": plan["candidate_commit"],
        "task_id": plan["task_id"],
        "manifest_tool_sha256": plan["tool_sha256"],
        "analysis_tool_sha256": file_sha256(Path(__file__)),
        "python_version": sys.version.split()[0],
        "primary_endpoint": endpoint,
        "random_stream_contract": plan["scenario_config"]["random_stream_contract"],
        "pairing_semantics": (
            "event-keyed common random numbers declared by the scenario contract"
            if plan["scenario_config"]["random_stream_contract"]["treatment_alignment"] == "event_keyed"
            else "paired by seed block, but common-random-number variance reduction is not assumed"
            if plan["scenario_config"]["random_stream_contract"]["treatment_alignment"] == "same_seed_only"
            else "scenario-declared alignment; inspect random_stream_contract.alignment_evidence"
        ),
        "positive_effect_means": "better according to preregistered primary_endpoint.direction",
        "held_out_seed_count": len(held_out),
        "bootstrap": {
            "replicates": bootstrap_replicates,
            "random_seed": bootstrap_seed,
            "interval": "95% percentile bootstrap over matched seed blocks",
            "multiplicity_adjustment": "none; intervals are descriptive and not confirmatory family-wise guarantees",
        },
        "variants": variants,
        "interpretation_boundary": "This report estimates effects within the specified simulator/task/configuration only. It does not establish empirical validity, consciousness, or a physical field. A complete manifest does not guarantee an adequate model.",
    }


def self_test():
    import unittest
    import tempfile

    class Tests(unittest.TestCase):
        def scenario(self, path):
            evidence_path = path.parent / "rng-alignment-evidence.txt"
            evidence_path.write_text(
                "Synthetic test fixture only: event IDs map deterministically to RNG values.",
                encoding="utf-8",
            )
            path.write_text(json.dumps({
                "task_description": "test scenario",
                "primary_endpoint": {"name": "utility", "units": "score", "direction": "higher_is_better",
                                     "meaningful_effect_threshold": 0.05},
                "solver_and_timestep": {"solver": "test", "dt": 0.016},
                "random_stream_contract": {
                    "strategy": "counter_based_event_keyed",
                    "treatment_alignment": "event_keyed",
                    "alignment_evidence": "Synthetic test fixture; not a claim about the Symtropy RNG.",
                    "alignment_evidence_path": evidence_path.name,
                    "alignment_evidence_sha256": file_sha256(evidence_path),
                },
            }), encoding="utf-8")
            return path

        def test_eight_arms_stable(self):
            self.assertEqual([a["arm_id"] for a in arms()], [f"A{i:03b}" for i in range(8)])

        def test_canonical_digest_and_nonfinite_rejection(self):
            self.assertEqual(digest({"b": 2, "a": 1}), digest({"a": 1, "b": 2}))
            with self.assertRaises(ValidationError):
                digest({"nan": float("nan")})

        def test_random_stream_contract_rejects_malformed_values_without_crashing(self):
            with tempfile.TemporaryDirectory() as tmp:
                scenario_path = self.scenario(Path(tmp) / "scenario.json")
                scenario = json.loads(scenario_path.read_text(encoding="utf-8"))
                scenario["random_stream_contract"]["strategy"] = []
                with self.assertRaises(ValidationError):
                    validate_scenario(scenario)
                scenario["random_stream_contract"]["strategy"] = "single_stateful_stream"
                scenario["random_stream_contract"]["treatment_alignment"] = ["same_seed_only"]
                with self.assertRaises(ValidationError):
                    validate_scenario(scenario)

        def test_plan_is_repeatable_and_complete(self):
            with tempfile.TemporaryDirectory() as tmp:
                scenario = self.scenario(Path(tmp) / "scenario.json")
                args = argparse.Namespace(candidate_commit="a"*40, task_id="smoke", scenario_config=scenario,
                                          seeds=[1,2,3], held_out_seeds=[3], dimensions=[2],
                                          source_strengths=[1.0], radii=[10.0])
                one, two = make_plan(args), make_plan(args)
                self.assertEqual(one["plan_sha256"], two["plan_sha256"])
                self.assertEqual(one["run_count"], 24)
                self.assertEqual(validate_plan(one), [])

        def test_incomplete_plan_is_rejected_even_when_rehashed(self):
            with tempfile.TemporaryDirectory() as tmp:
                scenario = self.scenario(Path(tmp) / "scenario.json")
                args = argparse.Namespace(candidate_commit="b"*40, task_id="smoke", scenario_config=scenario,
                                          seeds=[1,2], held_out_seeds=[2], dimensions=[3],
                                          source_strengths=[1.0], radii=[8.0])
                plan = make_plan(args)
                plan["planned_runs"].pop()
                plan["run_count"] -= 1
                unsigned = {k:v for k,v in plan.items() if k != "plan_sha256"}
                plan["plan_sha256"] = digest(unsigned)
                self.assertTrue(any("coverage mismatch" in e for e in validate_plan(plan)))

        def test_missing_results_do_not_qualify(self):
            with tempfile.TemporaryDirectory() as tmp:
                scenario = self.scenario(Path(tmp) / "scenario.json")
                args = argparse.Namespace(candidate_commit="c"*40, task_id="smoke", scenario_config=scenario,
                                          seeds=[1,2], held_out_seeds=[2], dimensions=[3],
                                          source_strengths=[1.0], radii=[8.0])
                plan = make_plan(args)
                result = Path(tmp) / "results.jsonl"
                result.write_text("", encoding="utf-8")
                report = validate_results(plan, result)
                self.assertEqual(report["validation_status"], "NOT_QUALIFIED")
                self.assertEqual(report["missing_run_count"], 16)

        def complete_records(self, plan, artifact_root):
            artifact_root.mkdir(parents=True, exist_ok=True)
            records = []
            for run in plan["planned_runs"]:
                trace_name = run["run_id"] + ".trace"
                replay_name = run["run_id"] + ".replay.json"
                trace = artifact_root / trace_name
                replay = artifact_root / replay_name
                trace.write_bytes(("trace:" + run["run_id"]).encode("utf-8"))
                replay.write_bytes(("replay:" + run["run_id"]).encode("utf-8"))
                records.append({
                    "schema_version": RESULT_SCHEMA,
                    "run_id": run["run_id"],
                    "candidate_commit": plan["candidate_commit"],
                    "plan_sha256": plan["plan_sha256"],
                    "configuration_sha256": run["configuration_sha256"],
                    "run_status": "completed",
                    "termination_reason": "task_complete",
                    "primary_endpoint": 1.0,
                    "secondary_metrics": {"collision_count": 0},
                    "validation_failure_count": 0,
                    "nonfinite_count": 0,
                    "invariant_drift": 0.0,
                    "artifact_path": trace_name,
                    "artifact_sha256": file_sha256(trace),
                    "replay_status": "exact_match",
                    "replay_evidence_path": replay_name,
                    "replay_evidence_sha256": file_sha256(replay),
                })
            return records

        def test_complete_manifest_passes_mechanical_checks_only(self):
            with tempfile.TemporaryDirectory() as tmp:
                scenario = self.scenario(Path(tmp) / "scenario.json")
                args = argparse.Namespace(candidate_commit="c"*40, task_id="smoke", scenario_config=scenario,
                                          seeds=[1,2], held_out_seeds=[2], dimensions=[3],
                                          source_strengths=[1.0], radii=[8.0])
                plan = make_plan(args)
                results = Path(tmp) / "results.jsonl"
                results.write_text("".join(json.dumps(r) + "\n" for r in self.complete_records(plan, Path(tmp))), encoding="utf-8")
                report = validate_results(plan, results, Path(tmp))
                self.assertEqual(report["validation_status"], "qualified_manifest")
                self.assertEqual(report["completed_run_count"], 16)
                self.assertIn("no effect estimates", report["note"])

        def test_nonzero_validation_counter_disqualifies_complete_manifest(self):
            with tempfile.TemporaryDirectory() as tmp:
                scenario = self.scenario(Path(tmp) / "scenario.json")
                args = argparse.Namespace(candidate_commit="d"*40, task_id="smoke", scenario_config=scenario,
                                          seeds=[1,2], held_out_seeds=[2], dimensions=[3],
                                          source_strengths=[1.0], radii=[8.0])
                plan = make_plan(args)
                records = self.complete_records(plan, Path(tmp))
                records[0]["validation_failure_count"] = 1
                results = Path(tmp) / "results.jsonl"
                results.write_text("".join(json.dumps(r) + "\n" for r in records), encoding="utf-8")
                report = validate_results(plan, results, Path(tmp))
                self.assertEqual(report["validation_status"], "NOT_QUALIFIED")
                self.assertEqual(report["completed_runs_with_invalid_counters"], 1)

        def test_analysis_recovers_synthetic_friction_only_effect(self):
            with tempfile.TemporaryDirectory() as tmp:
                scenario = self.scenario(Path(tmp) / "scenario.json")
                args = argparse.Namespace(candidate_commit="f"*40, task_id="synthetic-estimator-test",
                                          scenario_config=scenario, seeds=[1,2,3], held_out_seeds=[2,3],
                                          dimensions=[3], source_strengths=[1.0], radii=[8.0])
                plan = make_plan(args)
                records = self.complete_records(plan, Path(tmp))
                configs = {r["run_id"]: r["configuration"] for r in plan["planned_runs"]}
                for record in records:
                    cfg = configs[record["run_id"]]
                    enabled = cfg["mechanisms"]["friction_modulation"]
                    record["primary_endpoint"] = cfg["seed"] * 0.1 + (1.0 if enabled else 0.0)
                results = Path(tmp) / "results.jsonl"
                results.write_text("".join(json.dumps(r) + "\n" for r in records), encoding="utf-8")
                report = analyze_results(plan, results, Path(tmp), bootstrap_replicates=100, bootstrap_seed=23)
                self.assertEqual(report["analysis_status"], "analyzed_held_out_runs")
                self.assertEqual(report["held_out_seed_count"], 2)
                effects = report["variants"][0]["factorial_main_and_interaction_effects"]
                by_name = {effect["contrast"]: effect for effect in effects}
                self.assertAlmostEqual(by_name["F"]["effect_positive_is_better"], 1.0)
                self.assertEqual(by_name["F"]["bootstrap_95_percentile_interval"], [1.0, 1.0])
                self.assertAlmostEqual(by_name["R"]["effect_positive_is_better"], 0.0)
                self.assertAlmostEqual(by_name["E"]["effect_positive_is_better"], 0.0)
                arm_a100 = next(arm for arm in report["variants"][0]["arm_vs_A000"] if arm["arm_id"] == "A100")
                diagnostic = arm_a100["pairing_variance_diagnostic"]
                self.assertEqual(diagnostic["status"], "paired_variance_lower_than_independent_estimate")
                self.assertEqual(diagnostic["paired_to_independent_variance_ratio"], 0.0)

        def test_analysis_refuses_unqualified_manifest(self):
            with tempfile.TemporaryDirectory() as tmp:
                scenario = self.scenario(Path(tmp) / "scenario.json")
                args = argparse.Namespace(candidate_commit="a"*40, task_id="smoke",
                                          scenario_config=scenario, seeds=[1,2], held_out_seeds=[2],
                                          dimensions=[3], source_strengths=[1.0], radii=[8.0])
                plan = make_plan(args)
                result = Path(tmp) / "results.jsonl"
                result.write_text("", encoding="utf-8")
                report = analyze_results(plan, result, Path(tmp), bootstrap_replicates=100)
                self.assertEqual(report["analysis_status"], "NOT_QUALIFIED")

        def test_modified_random_stream_evidence_fails_qualification(self):
            with tempfile.TemporaryDirectory() as tmp:
                scenario = self.scenario(Path(tmp) / "scenario.json")
                args = argparse.Namespace(candidate_commit="9"*40, task_id="rng-evidence-test",
                                          scenario_config=scenario, seeds=[1,2], held_out_seeds=[2],
                                          dimensions=[3], source_strengths=[1.0], radii=[8.0])
                plan = make_plan(args)
                records = self.complete_records(plan, Path(tmp))
                evidence_path = Path(tmp) / plan["scenario_config"]["random_stream_contract"]["alignment_evidence_path"]
                evidence_path.write_text("altered evidence", encoding="utf-8")
                results = Path(tmp) / "results.jsonl"
                results.write_text("".join(json.dumps(r) + "\n" for r in records), encoding="utf-8")
                report = validate_results(plan, results, Path(tmp))
                self.assertEqual(report["validation_status"], "NOT_QUALIFIED")
                self.assertTrue(any("random-stream alignment evidence hash" in e for e in report["errors"]))

        def test_modified_artifact_bytes_fail_hash_verification(self):
            with tempfile.TemporaryDirectory() as tmp:
                scenario = self.scenario(Path(tmp) / "scenario.json")
                args = argparse.Namespace(candidate_commit="e"*40, task_id="smoke", scenario_config=scenario,
                                          seeds=[1,2], held_out_seeds=[2], dimensions=[3],
                                          source_strengths=[1.0], radii=[8.0])
                plan = make_plan(args)
                records = self.complete_records(plan, Path(tmp))
                (Path(tmp) / records[0]["artifact_path"]).write_bytes(b"modified after hash")
                results = Path(tmp) / "results.jsonl"
                results.write_text("".join(json.dumps(r) + "\n" for r in records), encoding="utf-8")
                report = validate_results(plan, results, Path(tmp))
                self.assertEqual(report["validation_status"], "NOT_QUALIFIED")
                self.assertTrue(any("does not match artifact bytes" in e for e in report["errors"]))

        def test_malformed_status_values_fail_closed_without_crashing(self):
            with tempfile.TemporaryDirectory() as tmp:
                scenario = self.scenario(Path(tmp) / "scenario.json")
                args = argparse.Namespace(candidate_commit="d"*40, task_id="smoke", scenario_config=scenario,
                                          seeds=[1,2], held_out_seeds=[2], dimensions=[3],
                                          source_strengths=[1.0], radii=[8.0])
                plan = make_plan(args)
                first = plan["planned_runs"][0]
                record = {"schema_version": RESULT_SCHEMA, "run_id": first["run_id"],
                          "candidate_commit": plan["candidate_commit"],
                          "plan_sha256": plan["plan_sha256"],
                          "configuration_sha256": first["configuration_sha256"],
                          "run_status": [], "termination_reason": "malformed",
                          "primary_endpoint": None, "validation_failure_count": 0,
                          "nonfinite_count": 0, "replay_status": []}
                result = Path(tmp) / "results.jsonl"
                result.write_text(json.dumps(record) + "\n", encoding="utf-8")
                report = validate_results(plan, result)
                self.assertEqual(report["validation_status"], "NOT_QUALIFIED")
                self.assertTrue(report["errors"])

    result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Tests))
    return 0 if result.wasSuccessful() else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("plan", help="create a deterministic planned manifest; performs no simulation")
    p.add_argument("--candidate-commit", required=True, help="full 40-character commit SHA being evaluated")
    p.add_argument("--task-id", required=True, help="predeclared benchmark/task identifier")
    p.add_argument("--scenario-config", required=True, type=Path, help="JSON with task, primary endpoint, solver, and timestep")
    p.add_argument("--seeds", required=True, type=lambda x: parse_list(x, int, "seeds", lambda v: v >= 0))
    p.add_argument("--held-out-seeds", required=True, type=lambda x: parse_list(x, int, "held-out seeds", lambda v: v >= 0))
    p.add_argument("--dimensions", required=True, type=lambda x: parse_list(x, int, "dimensions", lambda v: v > 0))
    p.add_argument("--source-strengths", required=True, type=lambda x: parse_list(x, float, "source strengths", lambda v: math.isfinite(v) and v >= 0))
    p.add_argument("--radii", required=True, type=lambda x: parse_list(x, float, "radii", lambda v: math.isfinite(v) and v > 0))
    p.add_argument("--output", required=True, type=Path)
    v = sub.add_parser("validate", help="validate one JSON result record per line against a plan")
    v.add_argument("--plan", required=True, type=Path)
    v.add_argument("--results", required=True, type=Path)
    v.add_argument("--report", required=True, type=Path)
    v.add_argument("--artifact-root", type=Path, help="root for relative artifact paths (default: results file directory)")
    a = sub.add_parser("analyze", help="estimate held-out paired arm and factorial effects; refuses unqualified manifests")
    a.add_argument("--plan", required=True, type=Path)
    a.add_argument("--results", required=True, type=Path)
    a.add_argument("--artifact-root", type=Path, help="root for relative artifact paths (default: results file directory)")
    a.add_argument("--output", required=True, type=Path)
    a.add_argument("--bootstrap-replicates", type=int, default=5000)
    a.add_argument("--bootstrap-seed", type=int, default=20261013)
    sub.add_parser("self-test", help="run the built-in dependency-free tests")
    args = parser.parse_args()
    if args.command == "self-test":
        return self_test()
    if args.command == "plan":
        try:
            plan = make_plan(args)
        except ValidationError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 2
        atomic_json(args.output, plan)
        print(json.dumps({"plan_sha256": plan["plan_sha256"], "run_count": plan["run_count"],
                          "seed_partition": plan["seed_partition"], "status": plan["plan_status"],
                          "output": str(args.output)}, sort_keys=True, indent=2))
        return 0
    try:
        plan = read_object(args.plan)
    except ValidationError as exc:
        print(json.dumps({"validation_status": "invalid_plan", "errors": [str(exc)]}, indent=2))
        return 2
    if args.command == "analyze":
        report = analyze_results(plan, args.results, args.artifact_root,
                                 args.bootstrap_replicates, args.bootstrap_seed)
        atomic_json(args.output, report)
        print(json.dumps(report, sort_keys=True, indent=2))
        return 0 if report.get("analysis_status") == "analyzed_held_out_runs" else 1
    report = validate_results(plan, args.results, args.artifact_root)
    atomic_json(args.report, report)
    print(json.dumps(report, sort_keys=True, indent=2))
    return 0 if report.get("validation_status") == "qualified_manifest" else 1


if __name__ == "__main__":
    raise SystemExit(main())
