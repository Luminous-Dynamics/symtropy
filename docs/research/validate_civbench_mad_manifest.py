import copy
import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

root = Path(__file__).resolve().parent
schema_path = root / "CIVBENCH_MEASUREMENT_DISTRIBUTION_V0_1.schema.json"
fixture_path = root / "fixtures" / "HUMSIM_SYNTHETIC_MAD_MANIFEST_V0_1.json"

schema = json.loads(schema_path.read_text(encoding="utf-8"))
manifest = json.loads(fixture_path.read_text(encoding="utf-8"))
Draft202012Validator.check_schema(schema)
validator = Draft202012Validator(schema, format_checker=FormatChecker())


def unique_values(records, key):
    values = [record[key] for record in records]
    return len(values) == len(set(values))


def semantic_errors(candidate):
    errors = []

    def unique(records, key, label):
        if not unique_values(records, key):
            errors.append(f"{label} values must be unique")

    unique(candidate["sources"], "source_id", "source_id")
    factors = candidate["evaluation_distribution"]["factors"]
    unique(factors, "factor_id", "factor_id")
    unique(candidate["perturbation_families"], "family_id", "perturbation family ID")
    unique(candidate["metrics"], "metric_id", "metric ID")
    unique(candidate["baselines"], "baseline_id", "baseline ID")
    unique(candidate["negative_controls"], "control_id", "negative-control ID")
    unique(candidate["holdouts"], "holdout_id", "holdout ID")

    for factor in factors:
        unique(factor["levels"], "level_id", f"{factor['factor_id']} level ID")
        weight_sum = sum(level["target_weight"] for level in factor["levels"])
        if abs(weight_sum - 1.0) > 1e-8:
            errors.append(
                f"{factor['factor_id']} target weights must sum to one; got {weight_sum}"
            )

    design = candidate["replication_design"]
    if design["inference_claimed"] and design["replication_count"] < 2:
        errors.append("inferential claims require more than one declared replication")
    if not design["inference_claimed"] and design["method"] == "deterministic-fixture":
        if design["replication_count"] != 1:
            errors.append("a deterministic fixture must declare exactly one execution")

    if candidate["composite_score_policy"] != "no-universal-composite":
        errors.append("CivBench does not permit a universal composite score")

    if candidate["construct"]["claim_tier"] == "tier-1-reference":
        if any(source["epistemic_status"] != "synthetic-test-fixture"
               for source in candidate["sources"]):
            errors.append("tier-1 reference fixtures must not silently mix empirical sources")
        if design["inference_claimed"]:
            errors.append("tier-1 synthetic reference fixtures cannot claim population inference")

    passport = candidate["configuration_passport"]
    if passport["configuration_id"] != candidate["evaluation_id"]:
        errors.append("configuration passport ID must match the evaluation ID")

    canonical_fields = {
        "benchmark_id": candidate["benchmark_id"],
        "evaluation_id": candidate["evaluation_id"],
        "configuration_id": passport["configuration_id"],
        "data_split_digest": passport["data_split_digest"],
        "model_id": passport["model"]["model_id"],
        "model_version": passport["model"]["model_version"],
        "task_contract_digest": passport["task_contract_digest"],
        **passport["facets"],
    }
    canonical_bytes = "\\n".join(
        f"{key}={canonical_fields[key] if canonical_fields[key] is not None else 'null'}"
        for key in sorted(canonical_fields)
    ).encode("utf-8")
    computed_digest = hashlib.sha256(canonical_bytes).hexdigest()
    if passport["configuration_digest"] != computed_digest:
        errors.append("configuration passport digest does not match canonical decision configuration")
    if passport["configuration_digest"] != candidate["reproduction"]["configuration_digest"]:
        errors.append("reproduction and configuration passport digests disagree")
    if passport["model"]["model_id"] != candidate["reproduction"]["model_identifier"]:
        errors.append("reproduction model identifier and configuration passport model disagree")
    if passport["data_split_digest"] != candidate["reproduction"]["data_manifest_digest"]:
        errors.append("configuration passport and reproduction data split digests disagree")

    stack = candidate["execution_stack"]
    components = stack["components"]
    unique(components, "component_id", "execution-stack component ID")
    component_by_id = {component["component_id"]: component for component in components}
    component_ids = set(component_by_id)

    for component in components:
        if component["behavior_affecting"] and not component["configuration_digest"]:
            errors.append(
                f"behavior-affecting component {component['component_id']} is not configuration-pinned"
            )

    required_component_roles = (
        (stack["scheduler_policy"]["scheduler_id"], "scheduler"),
        (stack["action_validation_policy"]["parser_component_id"], "action-parser"),
        (stack["action_validation_policy"]["validator_component_id"], "action-validator"),
        (stack["event_capture_policy"]["capture_component_id"], "event-capture"),
    )
    for component_id, expected_role in required_component_roles:
        if component_id not in component_ids:
            errors.append(f"execution-stack policy references unknown component: {component_id}")
        elif component_by_id[component_id]["role"] != expected_role:
            errors.append(
                f"execution-stack component {component_id} must have role {expected_role}"
            )

    if stack["exposure_policy"]["policy_id"] != "exposure-not-applicable":
        if not any(component["role"] == "exposure-policy" for component in components):
            errors.append("an active exposure policy needs an explicit exposure-policy component")

    if stack["action_validation_policy"]["retry_limit"] > 0:
        if not any(component["role"] == "retry-policy" for component in components):
            errors.append("a nonzero retry limit requires a declared retry-policy component")

    fallback_events = stack["fallback_events"]
    unique(fallback_events, "event_id", "fallback event ID")
    if stack["fallback_policy"] in ("none", "abort") and fallback_events:
        errors.append("fallback events are incompatible with the declared none/abort policy")
    if stack["fallback_policy"] == "counted-declared-default" and not fallback_events:
        errors.append("counted-declared-default requires explicit fallback event records")
    for event in fallback_events:
        component_id = event["component_id"]
        if component_id not in component_ids:
            errors.append(f"fallback event references unknown component: {component_id}")
        elif component_by_id[component_id]["role"] != "fallback":
            errors.append(f"fallback event component {component_id} must have role fallback")

    invalid_action_policy = stack["action_validation_policy"]["invalid_action_policy"]
    if invalid_action_policy == "counted-declared-default":
        if stack["fallback_policy"] != "counted-declared-default" or not fallback_events:
            errors.append("a counted default action requires matching fallback policy and event receipts")
    if stack["fallback_policy"] == "counted-declared-default" and invalid_action_policy != "counted-declared-default":
        errors.append("fallback policy and invalid-action policy disagree")

    unseeded = stack["unseeded_random_sources"]
    unique(unseeded, "source_id", "unseeded random source ID")
    for source in unseeded:
        component_id = source["component_id"]
        if component_id not in component_ids:
            errors.append(f"unseeded random source references unknown component: {component_id}")
        elif component_by_id[component_id]["role"] != "randomness":
            errors.append(f"random source component {component_id} must have role randomness")
    if unseeded and candidate["construct"]["claim_tier"] in (
        "tier-0-integrity",
        "tier-1-reference",
        "tier-2-empirical-distribution",
        "tier-3-external-generalization",
        "tier-4-intervention-validity",
    ):
        errors.append(
            "unseeded randomness is incompatible with a reproducibility/validity tier"
        )

    capture = stack["event_capture_policy"]
    required_event_kinds = capture["required_event_kinds"]
    if len(required_event_kinds) != len(set(required_event_kinds)):
        errors.append("required event kinds must be unique")
    if capture["dropped_event_count"] > 0 and capture["drop_policy"] == "fail-closed":
        errors.append("fail-closed event capture cannot report dropped events")

    return errors


def validate_candidate(candidate):
    schema_errors = list(validator.iter_errors(candidate))
    semantic = semantic_errors(candidate)
    return schema_errors, semantic


schema_errors, semantic = validate_candidate(manifest)
if schema_errors or semantic:
    details = [
        f"{list(error.absolute_path)}: {error.message}"
        for error in schema_errors
    ] + semantic
    raise SystemExit("Manifest does not conform:\\n" + "\\n".join(details))


def expect_semantic_rejection(candidate, expected_fragment):
    errors = semantic_errors(candidate)
    if not any(expected_fragment in error for error in errors):
        raise SystemExit(
            f"expected execution-stack negative control to reject with {expected_fragment!r}; got {errors}"
        )


# A decision-relevant persona change without a new matching digest must fail.
configuration_changed_without_reaudit = copy.deepcopy(manifest)
configuration_changed_without_reaudit["configuration_passport"]["facets"]["persona_id"] = "changed-persona"
expect_semantic_rejection(
    configuration_changed_without_reaudit,
    "configuration passport digest does not match",
)

# Negative controls demonstrate that the manifest validator itself detects
# hidden defaults, unseeded RNG, unbound scheduler IDs and silent event loss.
hidden_fallback = copy.deepcopy(manifest)
hidden_fallback["execution_stack"]["fallback_policy"] = "none"
hidden_fallback["execution_stack"]["fallback_events"] = [{
    "event_id": "fallback-001",
    "component_id": "typed-action-parser",
    "reason": "malformed model output",
    "action_taken": "silently substitute option zero",
}]
expect_semantic_rejection(hidden_fallback, "fallback events are incompatible")

unseeded_rng = copy.deepcopy(manifest)
unseeded_rng["execution_stack"]["unseeded_random_sources"] = [{
    "source_id": "rng-global",
    "component_id": "randomness-policy",
    "purpose": "unseeded agent action sampling",
}]
expect_semantic_rejection(unseeded_rng, "unseeded randomness is incompatible")

unbound_scheduler = copy.deepcopy(manifest)
unbound_scheduler["execution_stack"]["scheduler_policy"]["scheduler_id"] = "hidden-default-scheduler"
expect_semantic_rejection(unbound_scheduler, "unknown component")

silent_event_drop = copy.deepcopy(manifest)
silent_event_drop["execution_stack"]["event_capture_policy"]["dropped_event_count"] = 1
expect_semantic_rejection(silent_event_drop, "cannot report dropped events")

print(
    "CIVBENCH-MAD-001 schema, synthetic fixture, factor weights, identifiers, "
    "execution-stack references, fallback controls, RNG controls and claim ceiling validated."
)
