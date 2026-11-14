#!/usr/bin/env python3
"""Fail-closed schema and semantic checker for Humanity Atlas flourishing assessments.

The command-line entry point requires jsonschema==4.26.0 for Draft 2020-12
shape validation. Cross-record and artifact-integrity checks use the standard
library. This tool does not rank alternatives or certify a model or policy.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import sys
from datetime import datetime
from pathlib import Path, PurePosixPath
from typing import Any


SHA256_RE = re.compile(r"^[0-9a-fA-F]{64}$")


def _is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def _time(value: Any, label: str, errors: list[str]) -> datetime | None:
    if not isinstance(value, str):
        errors.append(f"{label}: expected an ISO-8601 timestamp")
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        errors.append(f"{label}: invalid ISO-8601 timestamp {value!r}")
        return None
    if parsed.tzinfo is None:
        errors.append(f"{label}: timestamp must include a timezone")
        return None
    return parsed


def _unique_records(records: Any, key: str, label: str, errors: list[str]) -> dict[str, dict[str, Any]]:
    if not isinstance(records, list):
        errors.append(f"{label}: expected an array")
        return {}
    by_id: dict[str, dict[str, Any]] = {}
    for index, record in enumerate(records):
        if not isinstance(record, dict):
            errors.append(f"{label}[{index}]: expected an object")
            continue
        identifier = record.get(key)
        if not isinstance(identifier, str) or not identifier:
            errors.append(f"{label}[{index}].{key}: expected a non-empty string")
            continue
        if identifier in by_id:
            errors.append(f"{label}: duplicate {key} {identifier!r}")
        else:
            by_id[identifier] = record
    return by_id


def _safe_relative_path(value: Any) -> bool:
    if (
        not isinstance(value, str)
        or not value
        or "\\" in value
        or any(ord(character) < 32 for character in value)
    ):
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and all(part not in ("", ".", "..") for part in path.parts)


def _validate_known_unknown(
    record: dict[str, Any], label: str, artifact_ids: set[str], errors: list[str]
) -> None:
    status = record.get("status")
    value = record.get("value")
    uncertainty = record.get("uncertainty")
    missing_reason = record.get("missing_reason")
    refs = record.get("artifact_ids")

    if not isinstance(refs, list) or any(not isinstance(ref, str) for ref in refs):
        errors.append(f"{label}.artifact_ids: expected an array of artifact IDs")
        refs = []
    if len(refs) != len(set(refs)):
        errors.append(f"{label}.artifact_ids: duplicate artifact references")
    for ref in refs:
        if ref not in artifact_ids:
            errors.append(f"{label}: references unknown artifact {ref!r}")

    if status == "KNOWN":
        if not _is_number(value):
            errors.append(f"{label}: KNOWN requires a finite numeric value")
        if not refs:
            errors.append(f"{label}: KNOWN requires at least one evidence artifact")
        if missing_reason is not None:
            errors.append(f"{label}: KNOWN requires missing_reason=null")
        if not isinstance(uncertainty, dict):
            errors.append(f"{label}: KNOWN requires explicit uncertainty characterization")
        else:
            uncertainty_status = uncertainty.get("status")
            method = uncertainty.get("method")
            if not isinstance(method, str) or len(method.strip()) < 3:
                errors.append(f"{label}.uncertainty.method: expected a non-empty method description")
            if uncertainty_status == "NOT_QUANTIFIED":
                reason = uncertainty.get("reason")
                if not isinstance(reason, str) or len(reason.strip()) < 8:
                    errors.append(f"{label}.uncertainty.reason: NOT_QUANTIFIED requires a reason")
                if any(field in uncertainty for field in ("lower", "upper", "interpretation", "coverage_probability")):
                    errors.append(f"{label}.uncertainty: NOT_QUANTIFIED cannot carry numeric bounds or interval metadata")
            elif uncertainty_status == "QUANTIFIED":
                if "reason" in uncertainty:
                    errors.append(f"{label}.uncertainty: QUANTIFIED must not include a not-quantified reason")
                low, high = uncertainty.get("lower"), uncertainty.get("upper")
                interpretation = uncertainty.get("interpretation")
                allowed_interpretations = {
                    "coverage_interval", "plausible_bounds", "scenario_envelope",
                    "sensitivity_range", "measurement_tolerance",
                }
                if not _is_number(low) or not _is_number(high) or low > high:
                    errors.append(f"{label}.uncertainty: bounds must be finite and lower <= upper")
                elif _is_number(value) and not low <= value <= high:
                    errors.append(f"{label}.uncertainty: bounds must contain the reported value")
                if interpretation not in allowed_interpretations:
                    errors.append(f"{label}.uncertainty.interpretation: unsupported or missing interpretation")
                coverage = uncertainty.get("coverage_probability")
                if interpretation == "coverage_interval":
                    if not _is_number(coverage) or not 0 < coverage <= 1:
                        errors.append(f"{label}.uncertainty.coverage_probability: coverage_interval requires a value in (0, 1]")
                elif "coverage_probability" in uncertainty:
                    errors.append(f"{label}.uncertainty.coverage_probability: only valid for coverage_interval")
            else:
                errors.append(f"{label}.uncertainty.status: expected QUANTIFIED or NOT_QUANTIFIED")
    elif status == "UNKNOWN":
        if value is not None:
            errors.append(f"{label}: UNKNOWN requires value=null")
        if uncertainty is not None:
            errors.append(f"{label}: UNKNOWN requires uncertainty=null")
        if not isinstance(missing_reason, str) or len(missing_reason.strip()) < 4:
            errors.append(f"{label}: UNKNOWN requires an explicit missing_reason")
    else:
        errors.append(f"{label}.status: expected KNOWN or UNKNOWN")


def _constraint_status(
    constraint: dict[str, Any], outcome: dict[str, Any], label: str,
    artifact_ids: set[str], verified_preregistration_ids: set[str], errors: list[str]
) -> str:
    targets: list[tuple[str, dict[str, Any]]] = [("overall", outcome)]
    scope = constraint.get("scope")
    if scope not in ("overall", "overall_and_subgroups"):
        errors.append(f"{label}.scope: expected overall or overall_and_subgroups")
        return "UNKNOWN"
    subgroups = outcome.get("subgroups", [])
    if scope == "overall_and_subgroups":
        if not isinstance(subgroups, list) or not subgroups:
            errors.append(f"{label}: scope overall_and_subgroups requires at least one subgroup record")
            return "UNKNOWN"
        targets.extend((f"subgroup:{g.get('group_id', '?')}", g) for g in subgroups if isinstance(g, dict))

    comparator = constraint.get("comparator")
    threshold = constraint.get("threshold")
    upper = constraint.get("threshold_upper")
    if comparator not in ("<=", ">=", "between"):
        errors.append(f"{label}.comparator: unsupported comparator {comparator!r}")
        return "UNKNOWN"
    if not _is_number(threshold):
        errors.append(f"{label}.threshold: expected a finite number")
        return "UNKNOWN"
    if comparator == "between":
        if not _is_number(upper) or threshold > upper:
            errors.append(f"{label}: between requires finite threshold <= threshold_upper")
            return "UNKNOWN"
    elif "threshold_upper" in constraint:
        errors.append(f"{label}: threshold_upper is only valid with comparator='between'")
        return "UNKNOWN"

    observed: list[tuple[str, dict[str, Any]]] = []
    for target_name, target in targets:
        _validate_known_unknown(target, f"{label}.{target_name}", artifact_ids, errors)
        if target.get("status") == "KNOWN" and _is_number(target.get("value")):
            observed.append((target_name, target))
    preregistration_id = constraint.get("preregistration_artifact_id")
    preregistration_missing = (
        not isinstance(preregistration_id, str)
        or not preregistration_id
        or preregistration_id not in verified_preregistration_ids
    )
    saw_unknown = preregistration_missing or any(
        target.get("status") == "UNKNOWN" for _, target in targets
    )

    failed = False
    uncertainty_overlap = False
    for _, target in observed:
        value = target["value"]
        interval = target.get("uncertainty")
        if not isinstance(interval, dict) or interval.get("status") != "QUANTIFIED":
            # A point estimate without quantified uncertainty is descriptive only;
            # it cannot establish that a non-compensable constraint is satisfied.
            uncertainty_overlap = True
            continue

        low, high = interval.get("lower"), interval.get("upper")
        if not _is_number(low) or not _is_number(high) or low > high:
            # Continue collecting validation errors instead of raising when this
            # function is called directly with malformed input outside the CLI.
            uncertainty_overlap = True
            continue
        # Treat the declared uncertainty envelope conservatively. If it overlaps a
        # failure region and does not lie wholly inside the permitted region, the
        # constraint is unresolved rather than passed.
        if comparator == "<=":
            if low > threshold:
                failed = True
            elif high > threshold:
                uncertainty_overlap = True
        elif comparator == ">=":
            if high < threshold:
                failed = True
            elif low < threshold:
                uncertainty_overlap = True
        else:  # validated comparator == "between"
            if high < threshold or low > upper:
                failed = True
            elif low < threshold or high > upper:
                uncertainty_overlap = True
    # A known violation is decisive even if another subgroup is unknown.
    if failed:
        return "FAIL"
    if uncertainty_overlap or saw_unknown or len(observed) != len(targets):
        return "UNKNOWN"
    return "PASS"


def validate_assessment(
    document: Any,
    *,
    artifact_root: Path | None = None,
    verify_artifacts: bool = False,
) -> tuple[list[str], dict[str, Any] | None]:
    """Return semantic errors and, if valid enough to evaluate, a non-ranking report."""
    errors: list[str] = []
    if not isinstance(document, dict):
        return ["assessment: expected a JSON object"], None

    if document.get("contract_version") != "0.1":
        errors.append("contract_version: expected '0.1'")

    declared_at = _time(document.get("declared_at"), "declared_at", errors)
    analysis_started_at = _time(document.get("analysis_started_at"), "analysis_started_at", errors)
    if declared_at and analysis_started_at and declared_at > analysis_started_at:
        errors.append("declared_at must be no later than analysis_started_at")

    decision = document.get("decision")
    if not isinstance(decision, dict):
        errors.append("decision: expected an object")
        decision = {}
    baseline_id = decision.get("baseline_alternative_id")

    metrics = _unique_records(document.get("metrics"), "metric_id", "metrics", errors)
    constraints = _unique_records(document.get("constraints"), "constraint_id", "constraints", errors)
    artifacts = _unique_records(document.get("artifacts"), "artifact_id", "artifacts", errors)
    alternatives = _unique_records(document.get("alternatives"), "alternative_id", "alternatives", errors)
    artifact_ids = set(artifacts)

    if len(alternatives) < 2:
        errors.append("alternatives: at least two alternatives are required")
    baselines = [a_id for a_id, alt in alternatives.items() if alt.get("kind") == "baseline"]
    if len(baselines) != 1:
        errors.append("alternatives: exactly one alternative must have kind='baseline'")
    if baseline_id not in alternatives or (baselines and baseline_id != baselines[0]):
        errors.append("decision.baseline_alternative_id must identify the unique baseline alternative")

    for constraint_id, constraint in constraints.items():
        metric_id = constraint.get("metric_id")
        if metric_id not in metrics:
            errors.append(f"constraints[{constraint_id}]: references unknown metric {metric_id!r}")
        elif metrics[metric_id].get("role") != "hard_constraint":
            errors.append(f"constraints[{constraint_id}]: metric {metric_id!r} is not registered as hard_constraint")
        if constraint.get("non_compensable") is not True:
            errors.append(f"constraints[{constraint_id}]: every registered hard constraint must be non-compensable")
        preregistration_id = constraint.get("preregistration_artifact_id")
        preregistration_collected_at = None
        if "preregistration_artifact_id" not in constraint:
            errors.append(f"constraints[{constraint_id}]: preregistration_artifact_id is required (use null when unproven)")
        elif preregistration_id is not None:
            if not isinstance(preregistration_id, str) or preregistration_id not in artifacts:
                errors.append(f"constraints[{constraint_id}]: references unknown preregistration artifact {preregistration_id!r}")
            else:
                preregistration = artifacts[preregistration_id]
                if preregistration.get("role") != "pre-registration":
                    errors.append(f"constraints[{constraint_id}]: referenced artifact role must be 'pre-registration'")
                preregistration_collected_at = _time(
                    preregistration.get("collected_at"),
                    f"artifacts[{preregistration_id}].collected_at",
                    errors,
                )
                if preregistration_collected_at and analysis_started_at and preregistration_collected_at > analysis_started_at:
                    errors.append(f"constraints[{constraint_id}]: preregistration artifact must predate analysis")
        predeclared = _time(constraint.get("predeclared_at"), f"constraints[{constraint_id}].predeclared_at", errors)
        if preregistration_collected_at and predeclared and preregistration_collected_at < predeclared:
            errors.append(f"constraints[{constraint_id}]: pre-registration artifact capture must be at or after predeclared_at")
        if predeclared and analysis_started_at and predeclared > analysis_started_at:
            errors.append(f"constraints[{constraint_id}]: must be predeclared no later than analysis_started_at")

    # Verify paths before reading bytes; a digest string alone is not evidence of integrity.
    verified_artifact_contents: dict[str, bytes] = {}
    verified_preregistration_ids: set[str] = set()
    preregistration_limit = 64 * 1024
    seen_artifact_paths: dict[str, str] = {}
    seen_resolved_paths: dict[str, str] = {}
    seen_digest_roles: dict[str, tuple[str, Any]] = {}
    for artifact_id, artifact in artifacts.items():
        relative = artifact.get("path")
        digest = artifact.get("sha256")
        if not _safe_relative_path(relative):
            errors.append(f"artifacts[{artifact_id}].path: must be a safe relative path without traversal")
            continue
        normalized_relative = str(PurePosixPath(relative))
        prior_artifact_id = seen_artifact_paths.get(normalized_relative)
        if prior_artifact_id is not None:
            errors.append(
                f"artifacts[{artifact_id}].path: duplicate artifact path also registered as {prior_artifact_id!r}"
            )
            continue
        seen_artifact_paths[normalized_relative] = artifact_id
        if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
            errors.append(f"artifacts[{artifact_id}].sha256: expected 64 hexadecimal characters")
            continue
        normalized_digest = digest.lower()
        role = artifact.get("role")
        prior_digest_role = seen_digest_roles.get(normalized_digest)
        if prior_digest_role is not None and prior_digest_role[1] != role:
            errors.append(
                f"artifacts[{artifact_id}].sha256: identical bytes already registered as "
                f"artifact {prior_digest_role[0]!r} under a different evidence role"
            )
        else:
            seen_digest_roles.setdefault(normalized_digest, (artifact_id, role))
        if verify_artifacts:
            if artifact_root is None:
                errors.append("artifact verification requested without an artifact root")
                break
            root = artifact_root.resolve()
            candidate = (root / relative).resolve()
            try:
                candidate.relative_to(root)
            except ValueError:
                errors.append(f"artifacts[{artifact_id}].path: resolves outside the artifact root")
                continue
            resolved_key = str(candidate)
            prior_resolved_id = seen_resolved_paths.get(resolved_key)
            if prior_resolved_id is not None:
                errors.append(
                    f"artifacts[{artifact_id}].path: resolves to the same file as artifact {prior_resolved_id!r}"
                )
                continue
            seen_resolved_paths[resolved_key] = artifact_id
            if not candidate.is_file():
                errors.append(f"artifacts[{artifact_id}]: file not found: {relative}")
                continue

            if artifact.get("role") == "pre-registration":
                try:
                    with candidate.open("rb") as stream:
                        payload = stream.read(preregistration_limit + 1)
                except OSError as exc:
                    errors.append(f"artifacts[{artifact_id}]: cannot read {relative}: {exc}")
                    continue
                if len(payload) > preregistration_limit:
                    errors.append(
                        f"artifacts[{artifact_id}]: pre-registration artifact exceeds {preregistration_limit} bytes"
                    )
                    continue
                actual_digest = hashlib.sha256(payload).hexdigest()
                if actual_digest.lower() != digest.lower():
                    errors.append(f"artifacts[{artifact_id}]: SHA-256 mismatch for {relative}")
                    continue
                verified_artifact_contents[artifact_id] = payload
            else:
                hasher = hashlib.sha256()
                try:
                    with candidate.open("rb") as stream:
                        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                            hasher.update(chunk)
                except OSError as exc:
                    errors.append(f"artifacts[{artifact_id}]: cannot read {relative}: {exc}")
                    continue
                if hasher.hexdigest().lower() != digest.lower():
                    errors.append(f"artifacts[{artifact_id}]: SHA-256 mismatch for {relative}")

    # A preregistration artifact must bind the actual constraint, not merely be
    # any digest-registered file labelled "pre-registration".
    required_prereg_fields = {
        "contract_version", "artifact_type", "constraint_id", "metric_id",
        "comparator", "threshold", "threshold_upper", "scope",
        "non_compensable", "declared_at", "rationale", "registered_by",
    }
    for constraint_id, constraint in constraints.items():
        preregistration_id = constraint.get("preregistration_artifact_id")
        if not isinstance(preregistration_id, str):
            continue
        preregistration_bytes = verified_artifact_contents.get(preregistration_id)
        if preregistration_bytes is None:
            # Missing/unverified bytes keep the constraint unresolved through its
            # tri-state result; other structural or hash errors already fail validation.
            continue
        try:
            preregistration_doc = json.loads(
                preregistration_bytes.decode("utf-8"),
                parse_constant=_reject_nonstandard_constant,
            )
        except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as exc:
            errors.append(
                f"constraints[{constraint_id}]: pre-registration artifact is not valid strict JSON: {exc}"
            )
            continue
        if not isinstance(preregistration_doc, dict) or set(preregistration_doc) != required_prereg_fields:
            errors.append(
                f"constraints[{constraint_id}]: pre-registration artifact fields must exactly match the v0.1 contract"
            )
            continue

        expected_upper = constraint.get("threshold_upper") if constraint.get("comparator") == "between" else None
        comparisons = {
            "contract_version": preregistration_doc.get("contract_version") == "0.1",
            "artifact_type": preregistration_doc.get("artifact_type") == "humanity_atlas_constraint_preregistration",
            "constraint_id": preregistration_doc.get("constraint_id") == constraint_id,
            "metric_id": preregistration_doc.get("metric_id") == constraint.get("metric_id"),
            "comparator": preregistration_doc.get("comparator") == constraint.get("comparator"),
            "threshold": _is_number(preregistration_doc.get("threshold"))
            and _is_number(constraint.get("threshold"))
            and preregistration_doc["threshold"] == constraint["threshold"],
            "threshold_upper": (
                preregistration_doc.get("threshold_upper") is None
                if expected_upper is None
                else _is_number(preregistration_doc.get("threshold_upper"))
                and _is_number(expected_upper)
                and preregistration_doc["threshold_upper"] == expected_upper
            ),
            "scope": preregistration_doc.get("scope") == constraint.get("scope"),
            "non_compensable": preregistration_doc.get("non_compensable") is True
            and constraint.get("non_compensable") is True,
            "rationale": isinstance(preregistration_doc.get("rationale"), str)
            and len(preregistration_doc["rationale"].strip()) >= 12,
            "registered_by": isinstance(preregistration_doc.get("registered_by"), str)
            and len(preregistration_doc["registered_by"].strip()) >= 3,
        }
        declared_at = _time(
            preregistration_doc.get("declared_at"),
            f"constraints[{constraint_id}].preregistration.declared_at",
            errors,
        )
        predeclared_at = _time(
            constraint.get("predeclared_at"),
            f"constraints[{constraint_id}].predeclared_at",
            errors,
        )
        comparisons["declared_at"] = (
            declared_at is not None and predeclared_at is not None and declared_at == predeclared_at
        )
        if not all(comparisons.values()):
            mismatched = sorted(key for key, matches in comparisons.items() if not matches)
            errors.append(
                f"constraints[{constraint_id}]: pre-registration artifact does not bind matching values for {mismatched}"
            )
        else:
            verified_preregistration_ids.add(preregistration_id)

    metric_ids = set(metrics)
    required_hard_metric_ids = {
        c.get("metric_id") for c in constraints.values() if isinstance(c.get("metric_id"), str)
    }
    for metric_id, metric in metrics.items():
        if metric.get("role") == "hard_constraint" and metric_id not in required_hard_metric_ids:
            errors.append(f"metrics[{metric_id}]: hard_constraint metric has no registered constraint")

    # Fair subgroup comparisons require the same declared group identities for
    # every alternative. Missing coverage must be represented as UNKNOWN rows,
    # not hidden by changing the set of groups between alternatives.
    subgroup_coverage_matches: dict[str, bool] = {}
    for constraint_id, constraint in constraints.items():
        if constraint.get("scope") != "overall_and_subgroups":
            subgroup_coverage_matches[constraint_id] = True
            continue
        metric_id = constraint.get("metric_id")
        group_sets: list[set[str]] = []
        for alternative in alternatives.values():
            outcome_map = {
                item.get("metric_id"): item for item in alternative.get("outcomes", [])
                if isinstance(item, dict)
            } if isinstance(alternative.get("outcomes"), list) else {}
            outcome = outcome_map.get(metric_id, {})
            groups = outcome.get("subgroups", []) if isinstance(outcome, dict) else []
            ids = {
                group.get("group_id") for group in groups
                if isinstance(group, dict) and isinstance(group.get("group_id"), str)
            } if isinstance(groups, list) else set()
            group_sets.append(ids)
        subgroup_coverage_matches[constraint_id] = bool(group_sets and group_sets[0]) and all(
            group_set == group_sets[0] for group_set in group_sets[1:]
        )

    report_alternatives: list[dict[str, Any]] = []
    for alt_id, alternative in alternatives.items():
        outcomes = _unique_records(alternative.get("outcomes"), "metric_id", f"alternatives[{alt_id}].outcomes", errors)
        if set(outcomes) != metric_ids:
            missing = sorted(metric_ids - set(outcomes))
            extra = sorted(set(outcomes) - metric_ids)
            if missing:
                errors.append(f"alternatives[{alt_id}].outcomes: missing metrics {missing}")
            if extra:
                errors.append(f"alternatives[{alt_id}].outcomes: unknown metrics {extra}")

        for metric_id, outcome in outcomes.items():
            if metric_id not in metrics:
                continue
            metric = metrics[metric_id]
            if outcome.get("unit") != metric.get("unit"):
                errors.append(f"alternatives[{alt_id}].outcomes[{metric_id}]: unit does not match metric registry")
            _validate_known_unknown(outcome, f"alternatives[{alt_id}].outcomes[{metric_id}]", artifact_ids, errors)
            outcome_artifact_refs = outcome.get("artifact_ids", [])
            if not isinstance(outcome_artifact_refs, list):
                outcome_artifact_refs = []
            for artifact_ref in outcome_artifact_refs:
                if artifact_ref in artifacts and artifacts[artifact_ref].get("role") == "pre-registration":
                    errors.append(
                        f"alternatives[{alt_id}].outcomes[{metric_id}]: pre-registration artifact cannot support a measured outcome"
                    )
            groups = outcome.get("subgroups", [])
            if not isinstance(groups, list):
                errors.append(f"alternatives[{alt_id}].outcomes[{metric_id}].subgroups: expected an array")
                continue
            group_ids = [g.get("group_id") for g in groups if isinstance(g, dict)]
            if len(group_ids) != len(set(group_ids)):
                errors.append(f"alternatives[{alt_id}].outcomes[{metric_id}].subgroups: duplicate group_id")
            for group_index, group in enumerate(groups):
                if not isinstance(group, dict):
                    errors.append(f"alternatives[{alt_id}].outcomes[{metric_id}].subgroups[{group_index}]: expected an object")
                    continue
                _validate_known_unknown(group, f"alternatives[{alt_id}].outcomes[{metric_id}].subgroup:{group.get('group_id', '?')}", artifact_ids, errors)
                group_artifact_refs = group.get("artifact_ids", [])
                if not isinstance(group_artifact_refs, list):
                    group_artifact_refs = []
                for artifact_ref in group_artifact_refs:
                    if artifact_ref in artifacts and artifacts[artifact_ref].get("role") == "pre-registration":
                        errors.append(
                            f"alternatives[{alt_id}].outcomes[{metric_id}].subgroup:{group.get('group_id', '?')}: pre-registration artifact cannot support a measured outcome"
                        )

        c_report: list[dict[str, str]] = []
        for constraint_id, constraint in constraints.items():
            metric_id = constraint.get("metric_id")
            outcome = outcomes.get(metric_id) if isinstance(metric_id, str) else None
            if outcome is None:
                status = "UNKNOWN"
            else:
                status = _constraint_status(
                    constraint,
                    outcome,
                    f"constraints[{constraint_id}]@{alt_id}",
                    artifact_ids,
                    verified_preregistration_ids,
                    errors,
                )
            coverage_matches = subgroup_coverage_matches.get(constraint_id, True)
            if (
                constraint.get("scope") == "overall_and_subgroups"
                and not coverage_matches
                and status == "PASS"
            ):
                status = "UNKNOWN"
            constraint_result = {"constraint_id": constraint_id, "status": status}
            if constraint.get("scope") == "overall_and_subgroups" and not coverage_matches:
                constraint_result["reason"] = "subgroup_coverage_mismatch"
            c_report.append(constraint_result)

        statuses = [record["status"] for record in c_report]
        any_incomplete_outcome = any(
            outcome.get("status") != "KNOWN"
            or not isinstance(outcome.get("uncertainty"), dict)
            or outcome["uncertainty"].get("status") != "QUANTIFIED"
            for outcome in outcomes.values()
        )
        if "FAIL" in statuses:
            comparison_status = "INELIGIBLE"
        elif "UNKNOWN" in statuses:
            comparison_status = "UNRESOLVED"
        elif any_incomplete_outcome or set(outcomes) != metric_ids:
            comparison_status = "OUTCOME_INCOMPLETE"
        else:
            comparison_status = "READY_FOR_COMPARISON"
        report_alternatives.append({
            "alternative_id": alt_id,
            "comparison_status": comparison_status,
            "constraint_results": c_report,
        })

    if errors:
        return errors, None

    return [], {
        "contract_version": "0.1",
        "assessment_id": document.get("assessment_id"),
        "assessment_status": "VALIDATED_ASSESSMENT_NOT_POLICY_APPROVAL",
        "artifact_bytes_verified": bool(verify_artifacts and artifacts),
        "ranking": None,
        "alternatives": report_alternatives,
        "limitations": [
            "This report checks declared structure, references, chronology, and artifact bytes only when artifact_bytes_verified is true.",
            "A hard constraint cannot pass unless its structured pre-registration artifact bytes were verified and matched to the declared constraint.",
            "PASS is relative to the registered threshold and evidence; it does not establish scientific validity or legal compliance.",
            "No candidate is ranked and no claim of real-world causal effect is made.",
        ],
    }


def _reject_nonstandard_constant(value: str) -> None:
    raise ValueError(f"non-standard JSON numeric constant is forbidden: {value}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("assessment", type=Path, help="assessment JSON file")
    parser.add_argument("--schema", type=Path, help="Draft 2020-12 schema (default: repository schema)")
    parser.add_argument("--artifact-root", type=Path, help="root for verifying artifact paths (default: assessment parent)")
    parser.add_argument("--report", type=Path, help="optional path for a JSON evaluation report")
    args = parser.parse_args(argv)

    try:
        with args.assessment.open("r", encoding="utf-8") as stream:
            document = json.load(stream, parse_constant=_reject_nonstandard_constant)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        print(f"ERROR: cannot read valid JSON assessment: {exc}", file=sys.stderr)
        return 2

    try:
        import jsonschema
    except ImportError:
        print("ERROR: schema validation dependency missing; install jsonschema==4.26.0", file=sys.stderr)
        return 2

    schema_path = args.schema or (
        Path(__file__).resolve().parents[2]
        / "docs/humanity-atlas/flourishing-assessment-v0.1.schema.json"
    )
    try:
        with schema_path.open("r", encoding="utf-8") as stream:
            schema = json.load(stream, parse_constant=_reject_nonstandard_constant)
        jsonschema.Draft202012Validator.check_schema(schema)
        shape_validator = jsonschema.Draft202012Validator(
            schema, format_checker=jsonschema.FormatChecker()
        )
        schema_errors = sorted(
            shape_validator.iter_errors(document),
            key=lambda err: tuple(str(part) for part in err.absolute_path),
        )
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        print(f"ERROR: cannot read schema {schema_path}: {exc}", file=sys.stderr)
        return 2
    except Exception as exc:
        print(f"ERROR: cannot initialize Draft 2020-12 schema validator: {exc}", file=sys.stderr)
        return 2

    if schema_errors:
        for error in schema_errors:
            path = ".".join(str(part) for part in error.absolute_path) or "<root>"
            print(f"ERROR: schema {path}: {error.message}", file=sys.stderr)
        return 1

    root = args.artifact_root or args.assessment.parent
    errors, report = validate_assessment(document, artifact_root=root, verify_artifacts=True)
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    rendered = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
    if args.report:
        report_target = args.report.resolve(strict=False)
        protected_targets = {args.assessment.resolve(strict=False), schema_path.resolve(strict=False)}
        artifact_root_for_paths = (args.artifact_root or args.assessment.parent).resolve()
        for artifact in document.get("artifacts", []):
            if not isinstance(artifact, dict):
                continue
            relative = artifact.get("path")
            if _safe_relative_path(relative):
                protected_targets.add((artifact_root_for_paths / relative).resolve(strict=False))
        if report_target in protected_targets:
            print("ERROR: report path would overwrite the assessment, schema, or a registered evidence artifact", file=sys.stderr)
            return 2
        try:
            args.report.write_text(rendered, encoding="utf-8")
        except OSError as exc:
            print(f"ERROR: cannot write report: {exc}", file=sys.stderr)
            return 2
    print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
