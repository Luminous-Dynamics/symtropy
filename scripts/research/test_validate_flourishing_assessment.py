"""Regression tests for fail-closed Humanity Atlas flourishing assessments."""
from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path

try:
    import jsonschema
except ImportError:  # CI installs the pinned schema validator.
    jsonschema = None

ROOT = Path(__file__).resolve().parents[2]
SCHEMA_PATH = ROOT / "docs/humanity-atlas/flourishing-assessment-v0.1.schema.json"
PREREG_SCHEMA_PATH = ROOT / "docs/humanity-atlas/constraint-preregistration-v0.1.schema.json"
sys.path.insert(0, str(Path(__file__).resolve().parent))
from validate_flourishing_assessment import main, validate_assessment  # noqa: E402


def unknown_outcome(metric_id: str, unit: str) -> dict:
    return {
        "metric_id": metric_id,
        "status": "UNKNOWN",
        "value": None,
        "unit": unit,
        "uncertainty": None,
        "missing_reason": "no qualifying measurement is available",
        "artifact_ids": [],
        "subgroups": [],
    }


def assessment() -> dict:
    return {
        "contract_version": "0.1",
        "assessment_id": "test-flourishing-v01",
        "declared_at": "2026-10-13T09:00:00Z",
        "analysis_started_at": "2026-10-13T11:00:00Z",
        "decision": {
            "question": "Which service plan best protects reliable access?",
            "intended_use": "exploratory_simulation",
            "decision_owner": "test steward",
            "affected_population": "synthetic example population",
            "jurisdiction": "synthetic jurisdiction",
            "time_horizon": "12 months",
            "baseline_alternative_id": "baseline",
        },
        "metrics": [
            {
                "metric_id": "outage_hours",
                "definition": "Total service interruption hours per month",
                "unit": "hours/month",
                "direction": "lower_is_better",
                "role": "outcome",
                "measurement_method": "interval meter aggregation",
                "scope": "service-area aggregate",
                "missingness_policy": "report unknown if required telemetry is absent",
                "privacy_classification": "aggregated",
            },
            {
                "metric_id": "critical_outage_hours",
                "definition": "Critical interruption hours per month",
                "unit": "hours/month",
                "direction": "lower_is_better",
                "role": "hard_constraint",
                "measurement_method": "interval meter aggregation",
                "scope": "service-area aggregate",
                "missingness_policy": "unknown evidence never satisfies this constraint",
                "privacy_classification": "aggregated",
            },
        ],
        "constraints": [
            {
                "constraint_id": "critical-outage-limit",
                "metric_id": "critical_outage_hours",
                "comparator": "<=",
                "threshold": 1.0,
                "scope": "overall",
                "non_compensable": True,
                "predeclared_at": "2026-10-13T10:00:00Z",
                "preregistration_artifact_id": None,
            }
        ],
        "artifacts": [],
        "alternatives": [
            {
                "alternative_id": "baseline",
                "label": "Current service",
                "kind": "baseline",
                "outcomes": [
                    unknown_outcome("outage_hours", "hours/month"),
                    unknown_outcome("critical_outage_hours", "hours/month"),
                ],
            },
            {
                "alternative_id": "candidate",
                "label": "Candidate service plan",
                "kind": "intervention",
                "outcomes": [
                    unknown_outcome("outage_hours", "hours/month"),
                    unknown_outcome("critical_outage_hours", "hours/month"),
                ],
            },
        ],
        "limitations": {
            "assumptions": ["Synthetic example only"],
            "known_data_gaps": ["No measurements have been supplied"],
            "excluded_impacts": ["Household affordability"],
            "reviewer_conflicts": [],
        },
    }


def preregistration_payload(constraint: dict) -> bytes:
    payload = {
        "contract_version": "0.1",
        "artifact_type": "humanity_atlas_constraint_preregistration",
        "constraint_id": constraint["constraint_id"],
        "metric_id": constraint["metric_id"],
        "comparator": constraint["comparator"],
        "threshold": constraint["threshold"],
        "threshold_upper": constraint.get("threshold_upper") if constraint["comparator"] == "between" else None,
        "scope": constraint["scope"],
        "non_compensable": constraint["non_compensable"],
        "declared_at": constraint["predeclared_at"],
        "rationale": "Synthetic preregistration protects essential service continuity.",
        "registered_by": "synthetic test steward",
    }
    return (json.dumps(payload, indent=2) + "\n").encode("utf-8")


def materialize_artifact_files(doc: dict, root: Path) -> None:
    root.mkdir(parents=True, exist_ok=True)
    (root / "sample.txt").write_bytes(b"synthetic evidence; not a real observation\n")
    for constraint in doc["constraints"]:
        artifact_id = constraint.get("preregistration_artifact_id")
        if not isinstance(artifact_id, str):
            continue
        artifact = next(a for a in doc["artifacts"] if a["artifact_id"] == artifact_id)
        (root / artifact["path"]).write_bytes(preregistration_payload(constraint))


def refresh_preregistration_artifacts(doc: dict) -> None:
    for constraint in doc["constraints"]:
        artifact_id = constraint.get("preregistration_artifact_id")
        if not isinstance(artifact_id, str):
            continue
        artifact = next(a for a in doc["artifacts"] if a["artifact_id"] == artifact_id)
        payload = preregistration_payload(constraint)
        artifact["sha256"] = hashlib.sha256(payload).hexdigest()
        artifact["collected_at"] = constraint["predeclared_at"]


def validate_with_artifacts(doc: dict) -> tuple[list[str], dict | None]:
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        materialize_artifact_files(doc, root)
        return validate_assessment(doc, artifact_root=root, verify_artifacts=True)


def add_evidence_and_known_values(doc: dict, values: dict[str, float] | None = None) -> dict:
    raw = b"synthetic evidence; not a real observation\n"
    doc["artifacts"] = [{
        "artifact_id": "sample-artifact",
        "path": "sample.txt",
        "sha256": hashlib.sha256(raw).hexdigest(),
        "role": "outcome-evidence",
        "purpose": "Synthetic verifier test input; contents are test-only.",
        "collected_at": "2026-10-13T10:30:00Z",
    }]
    for index, constraint in enumerate(doc["constraints"]):
        artifact_id = f"preregistration-artifact-{index}"
        artifact_path = f"preregistration-{index}.json"
        constraint["preregistration_artifact_id"] = artifact_id
        preregistration_bytes = preregistration_payload(constraint)
        doc["artifacts"].append({
            "artifact_id": artifact_id,
            "path": artifact_path,
            "sha256": hashlib.sha256(preregistration_bytes).hexdigest(),
            "role": "pre-registration",
            "purpose": "Constraint threshold pre-registration test fixture.",
            "collected_at": constraint["predeclared_at"],
        })
    values = values or {"outage_hours": 3.0, "critical_outage_hours": 0.5}
    for alternative in doc["alternatives"]:
        for outcome in alternative["outcomes"]:
            outcome["status"] = "KNOWN"
            value = values.get(outcome["metric_id"], 0.5)
            outcome["value"] = value
            outcome["uncertainty"] = {
                "status": "QUANTIFIED",
                "lower": value,
                "upper": value,
                "interpretation": "scenario_envelope",
                "method": "synthetic deterministic test value",
            }
            outcome["missing_reason"] = None
            outcome["artifact_ids"] = ["sample-artifact"]
    return doc


class FlourishingAssessmentTests(unittest.TestCase):
    def test_unknown_hard_constraint_remains_unresolved(self) -> None:
        errors, report = validate_assessment(assessment())
        self.assertEqual(errors, [])
        self.assertIsNotNone(report)
        by_id = {a["alternative_id"]: a for a in report["alternatives"]}
        self.assertEqual(by_id["candidate"]["comparison_status"], "UNRESOLVED")
        self.assertEqual(by_id["candidate"]["constraint_results"][0]["status"], "UNKNOWN")
        self.assertIsNone(report["ranking"])

    def test_unverified_artifact_bytes_never_qualify_a_hard_constraint(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        errors, report = validate_assessment(doc)
        self.assertEqual(errors, [])
        self.assertFalse(report["artifact_bytes_verified"])
        by_id = {a["alternative_id"]: a for a in report["alternatives"]}
        self.assertEqual(by_id["candidate"]["comparison_status"], "UNRESOLVED")
        self.assertEqual(by_id["candidate"]["constraint_results"][0]["status"], "UNKNOWN")

    def test_known_passing_constraints_produce_comparison_readiness_not_a_ranking(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        errors, report = validate_with_artifacts(doc)
        self.assertEqual(errors, [])
        self.assertEqual(
            {a["comparison_status"] for a in report["alternatives"]},
            {"READY_FOR_COMPARISON"},
        )
        self.assertIsNone(report["ranking"])

    def test_hard_constraint_failure_makes_candidate_ineligible(self) -> None:
        doc = add_evidence_and_known_values(
            assessment(), {"outage_hours": 3.0, "critical_outage_hours": 1.2}
        )
        errors, report = validate_with_artifacts(doc)
        self.assertEqual(errors, [])
        by_id = {a["alternative_id"]: a for a in report["alternatives"]}
        self.assertEqual(by_id["candidate"]["comparison_status"], "INELIGIBLE")
        self.assertEqual(by_id["candidate"]["constraint_results"][0]["status"], "FAIL")

    def test_uncertainty_crossing_constraint_boundary_is_unresolved(self) -> None:
        doc = add_evidence_and_known_values(
            assessment(), {"outage_hours": 3.0, "critical_outage_hours": 0.9}
        )
        candidate_constraint = doc["alternatives"][1]["outcomes"][1]
        candidate_constraint["uncertainty"] = {
            "status": "QUANTIFIED",
            "lower": 0.5,
            "upper": 1.2,
            "interpretation": "scenario_envelope",
            "method": "synthetic uncertainty envelope",
        }
        errors, report = validate_with_artifacts(doc)
        self.assertEqual(errors, [])
        by_id = {a["alternative_id"]: a for a in report["alternatives"]}
        self.assertEqual(by_id["candidate"]["constraint_results"][0]["status"], "UNKNOWN")
        self.assertEqual(by_id["candidate"]["comparison_status"], "UNRESOLVED")

    def test_uncertainty_envelope_inside_constraint_can_pass(self) -> None:
        doc = add_evidence_and_known_values(
            assessment(), {"outage_hours": 3.0, "critical_outage_hours": 0.5}
        )
        for alternative in doc["alternatives"]:
            alternative["outcomes"][1]["uncertainty"] = {
                "status": "QUANTIFIED",
                "lower": 0.2,
                "upper": 0.8,
                "interpretation": "scenario_envelope",
                "method": "synthetic uncertainty envelope",
            }
        errors, report = validate_with_artifacts(doc)
        self.assertEqual(errors, [])
        self.assertEqual(
            {a["comparison_status"] for a in report["alternatives"]},
            {"READY_FOR_COMPARISON"},
        )

    def test_unquantified_hard_constraint_remains_unresolved(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        candidate_constraint = doc["alternatives"][1]["outcomes"][1]
        candidate_constraint["uncertainty"] = {
            "status": "NOT_QUANTIFIED",
            "method": "uncertainty budget assessment",
            "reason": "no defensible interval is available",
        }
        errors, report = validate_with_artifacts(doc)
        self.assertEqual(errors, [])
        by_id = {a["alternative_id"]: a for a in report["alternatives"]}
        self.assertEqual(by_id["candidate"]["constraint_results"][0]["status"], "UNKNOWN")
        self.assertEqual(by_id["candidate"]["comparison_status"], "UNRESOLVED")

    def test_missing_preregistration_evidence_prevents_constraint_pass(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        doc["constraints"][0]["preregistration_artifact_id"] = None
        errors, report = validate_assessment(doc)
        self.assertEqual(errors, [])
        by_id = {a["alternative_id"]: a for a in report["alternatives"]}
        self.assertEqual(by_id["candidate"]["constraint_results"][0]["status"], "UNKNOWN")
        self.assertEqual(by_id["candidate"]["comparison_status"], "UNRESOLVED")

    def test_preregistration_artifact_must_precede_analysis(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        preregistration_id = doc["constraints"][0]["preregistration_artifact_id"]
        matching_artifacts = [
            artifact for artifact in doc["artifacts"]
            if artifact["artifact_id"] == preregistration_id
        ]
        self.assertEqual(len(matching_artifacts), 1, "constraint must reference one pre-registration artifact")
        matching_artifacts[0]["collected_at"] = "2026-10-13T12:00:00Z"
        errors, report = validate_assessment(doc)
        self.assertTrue(any("preregistration artifact must predate analysis" in error for error in errors))
        self.assertIsNone(report)

    def test_unquantified_non_constraint_outcome_is_incomplete(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        for alternative in doc["alternatives"]:
            alternative["outcomes"][0]["uncertainty"] = {
                "status": "NOT_QUANTIFIED",
                "method": "uncertainty budget assessment",
                "reason": "no defensible interval is available",
            }
        errors, report = validate_with_artifacts(doc)
        self.assertEqual(errors, [])
        self.assertEqual(
            {a["comparison_status"] for a in report["alternatives"]},
            {"OUTCOME_INCOMPLETE"},
        )

    def test_invalid_between_thresholds_rejected_even_when_values_unknown(self) -> None:
        doc = assessment()
        doc["constraints"][0]["comparator"] = "between"
        doc["constraints"][0]["threshold"] = 2.0
        doc["constraints"][0]["threshold_upper"] = 1.0
        errors, report = validate_assessment(doc)
        self.assertTrue(any("between requires finite threshold" in error for error in errors))
        self.assertIsNone(report)

    def test_malformed_quantified_interval_returns_validation_error(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        del doc["alternatives"][1]["outcomes"][1]["uncertainty"]["lower"]
        errors, report = validate_assessment(doc)
        self.assertTrue(any("bounds must be finite" in error for error in errors))
        self.assertIsNone(report)

    def test_missing_metric_outcome_is_rejected(self) -> None:
        doc = assessment()
        doc["alternatives"][0]["outcomes"].pop()
        errors, report = validate_assessment(doc)
        self.assertTrue(any("missing metrics" in error for error in errors))
        self.assertIsNone(report)

    def test_duplicate_metric_ids_are_rejected(self) -> None:
        doc = assessment()
        doc["metrics"].append(copy.deepcopy(doc["metrics"][0]))
        errors, _ = validate_assessment(doc)
        self.assertTrue(any("duplicate metric_id" in error for error in errors))

    def test_constraint_must_be_preregistered_before_analysis(self) -> None:
        doc = assessment()
        doc["constraints"][0]["predeclared_at"] = "2026-10-13T12:00:00Z"
        errors, _ = validate_assessment(doc)
        self.assertTrue(any("predeclared no later" in error for error in errors))

    def test_unknown_outcome_cannot_contain_a_numeric_value(self) -> None:
        doc = assessment()
        doc["alternatives"][0]["outcomes"][0]["value"] = 0
        errors, _ = validate_assessment(doc)
        self.assertTrue(any("UNKNOWN requires value=null" in error for error in errors))

    def test_artifact_path_traversal_is_rejected(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        doc["artifacts"][0]["path"] = "../outside.txt"
        errors, _ = validate_assessment(doc)
        self.assertTrue(any("safe relative path" in error for error in errors))

    def test_artifact_path_control_characters_are_rejected(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        doc["artifacts"][0]["path"] = "sample.txt\x00suffix"
        errors, _ = validate_assessment(doc)
        self.assertTrue(any("safe relative path" in error for error in errors))

    def test_duplicate_artifact_path_cannot_change_evidence_role(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        doc["artifacts"][0]["path"] = doc["artifacts"][1]["path"]
        errors, _ = validate_assessment(doc)
        self.assertTrue(any("duplicate artifact path" in error for error in errors))

    def test_symlink_alias_cannot_reuse_preregistration_bytes_as_outcome_evidence(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            materialize_artifact_files(doc, root)
            prereg_artifact = next(
                artifact for artifact in doc["artifacts"]
                if artifact["role"] == "pre-registration"
            )
            prereg_path = root / prereg_artifact["path"]
            sample_artifact = next(
                artifact for artifact in doc["artifacts"]
                if artifact["artifact_id"] == "sample-artifact"
            )
            sample_path = root / sample_artifact["path"]
            prereg_bytes = prereg_path.read_bytes()
            sample_artifact["sha256"] = hashlib.sha256(prereg_bytes).hexdigest()
            sample_path.unlink()
            sample_path.symlink_to(prereg_path.name)
            errors, report = validate_assessment(
                doc, artifact_root=root, verify_artifacts=True
            )
        self.assertTrue(
            any("resolves to the same file as artifact" in error for error in errors)
        )
        self.assertIsNone(report)

    def test_copied_preregistration_bytes_cannot_be_relabelled_as_outcome_evidence(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            materialize_artifact_files(doc, root)
            prereg_artifact = next(
                artifact for artifact in doc["artifacts"]
                if artifact["role"] == "pre-registration"
            )
            sample_artifact = next(
                artifact for artifact in doc["artifacts"]
                if artifact["artifact_id"] == "sample-artifact"
            )
            prereg_bytes = (root / prereg_artifact["path"]).read_bytes()
            (root / sample_artifact["path"]).write_bytes(prereg_bytes)
            sample_artifact["sha256"] = hashlib.sha256(prereg_bytes).hexdigest()
            errors, report = validate_assessment(
                doc, artifact_root=root, verify_artifacts=True
            )
        self.assertTrue(
            any("identical bytes already registered" in error and "different evidence role" in error
                for error in errors)
        )
        self.assertIsNone(report)

    def test_artifact_digest_is_checked_against_file_bytes(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            materialize_artifact_files(doc, root)
            (root / "sample.txt").write_bytes(b"bytes differ from the declared digest")
            errors, _ = validate_assessment(doc, artifact_root=root, verify_artifacts=True)
        self.assertTrue(any("SHA-256 mismatch" in error for error in errors))

    def test_preregistration_content_must_match_constraint_even_with_valid_digest(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        artifact_id = doc["constraints"][0]["preregistration_artifact_id"]
        artifact = next(item for item in doc["artifacts"] if item["artifact_id"] == artifact_id)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            materialize_artifact_files(doc, root)
            path = root / artifact["path"]
            payload = json.loads(path.read_text(encoding="utf-8"))
            payload["threshold"] = 0.75
            tampered_but_rehashed = (json.dumps(payload, indent=2) + "\n").encode("utf-8")
            path.write_bytes(tampered_but_rehashed)
            artifact["sha256"] = hashlib.sha256(tampered_but_rehashed).hexdigest()
            errors, report = validate_assessment(doc, artifact_root=root, verify_artifacts=True)
        self.assertTrue(any("does not bind matching values" in error and "threshold" in error for error in errors))
        self.assertIsNone(report)

    def test_preregistration_artifact_cannot_be_cited_as_outcome_evidence(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        prereg_id = doc["constraints"][0]["preregistration_artifact_id"]
        doc["alternatives"][0]["outcomes"][0]["artifact_ids"] = [prereg_id]
        errors, report = validate_assessment(doc)
        self.assertTrue(any("cannot support a measured outcome" in error for error in errors))
        self.assertIsNone(report)

    def test_subgroup_coverage_mismatch_prevents_constraint_pass(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        constraint = doc["constraints"][0]
        constraint["scope"] = "overall_and_subgroups"
        refresh_preregistration_artifacts(doc)
        for alternative, group_id in zip(doc["alternatives"], ["area-a", "area-b"]):
            outcome = alternative["outcomes"][1]
            outcome["subgroups"] = [{
                "group_id": group_id,
                "status": "KNOWN",
                "value": 0.5,
                "uncertainty": {
                    "status": "QUANTIFIED",
                    "lower": 0.5,
                    "upper": 0.5,
                    "interpretation": "scenario_envelope",
                    "method": "synthetic subgroup value",
                },
                "missing_reason": None,
                "artifact_ids": ["sample-artifact"],
            }]
        errors, report = validate_with_artifacts(doc)
        self.assertEqual(errors, [])
        for alternative in report["alternatives"]:
            self.assertEqual(alternative["constraint_results"][0]["status"], "UNKNOWN")
            self.assertEqual(alternative["constraint_results"][0]["reason"], "subgroup_coverage_mismatch")
            self.assertEqual(alternative["comparison_status"], "UNRESOLVED")

    def test_subgroup_violation_is_not_hidden_by_an_unknown_subgroup(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        constraint = doc["constraints"][0]
        constraint["scope"] = "overall_and_subgroups"
        refresh_preregistration_artifacts(doc)
        baseline = doc["alternatives"][0]["outcomes"][1]
        baseline["subgroups"] = [
            {
                "group_id": "area-baseline",
                "status": "UNKNOWN",
                "value": None,
                "uncertainty": None,
                "missing_reason": "subgroup telemetry is unavailable",
                "artifact_ids": [],
            }
        ]
        candidate = doc["alternatives"][1]["outcomes"][1]
        candidate["subgroups"] = [
            {
                "group_id": "area-a",
                "status": "KNOWN",
                "value": 1.4,
                "uncertainty": {
                    "status": "QUANTIFIED",
                    "lower": 1.4,
                    "upper": 1.4,
                    "interpretation": "scenario_envelope",
                    "method": "synthetic subgroup value",
                },
                "missing_reason": None,
                "artifact_ids": ["sample-artifact"],
            },
            {
                "group_id": "area-b",
                "status": "UNKNOWN",
                "value": None,
                "uncertainty": None,
                "missing_reason": "sensor coverage is unavailable",
                "artifact_ids": [],
            },
        ]
        errors, report = validate_with_artifacts(doc)
        self.assertEqual(errors, [])
        by_id = {a["alternative_id"]: a for a in report["alternatives"]}
        self.assertEqual(by_id["candidate"]["constraint_results"][0]["status"], "FAIL")
        self.assertEqual(by_id["candidate"]["comparison_status"], "INELIGIBLE")

    def test_checked_in_not_evidence_fixture_remains_unresolved(self) -> None:
        fixture_path = ROOT / "docs/humanity-atlas/fixtures/flourishing-assessment.not-evidence.json"
        doc = json.loads(fixture_path.read_text(encoding="utf-8"))
        errors, report = validate_assessment(doc)
        self.assertEqual(errors, [])
        self.assertEqual(report["assessment_status"], "VALIDATED_ASSESSMENT_NOT_POLICY_APPROVAL")
        self.assertIsNone(report["ranking"])
        self.assertEqual(
            {a["comparison_status"] for a in report["alternatives"]},
            {"UNRESOLVED"},
        )

    @unittest.skipIf(jsonschema is None, "jsonschema dependency is installed by the CI workflow")
    def test_cli_validates_checked_in_fixture_and_reports_unresolved(self) -> None:
        fixture_path = ROOT / "docs/humanity-atlas/fixtures/flourishing-assessment.not-evidence.json"
        output = StringIO()
        with redirect_stdout(output):
            exit_code = main([str(fixture_path)])
        self.assertEqual(exit_code, 0)
        report = json.loads(output.getvalue())
        self.assertFalse(report["artifact_bytes_verified"])
        self.assertIsNone(report["ranking"])
        self.assertEqual(report["assessment_status"], "VALIDATED_ASSESSMENT_NOT_POLICY_APPROVAL")
        self.assertEqual(
            {item["comparison_status"] for item in report["alternatives"]},
            {"UNRESOLVED"},
        )

    @unittest.skipIf(jsonschema is None, "jsonschema dependency is installed by the CI workflow")
    def test_cli_refuses_to_overwrite_assessment_with_report(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "assessment.json"
            original = json.dumps(assessment(), indent=2) + "\n"
            path.write_text(original, encoding="utf-8")
            error_output = StringIO()
            with redirect_stderr(error_output):
                exit_code = main([str(path), "--report", str(path)])
            self.assertEqual(exit_code, 2)
            self.assertIn("would overwrite the assessment", error_output.getvalue())
            self.assertEqual(path.read_text(encoding="utf-8"), original)

    @unittest.skipIf(jsonschema is None, "jsonschema dependency is installed by the CI workflow")
    def test_cli_refuses_to_overwrite_registered_evidence(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            materialize_artifact_files(doc, root)
            assessment_path = root / "assessment.json"
            assessment_path.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
            evidence_path = root / "sample.txt"
            original_evidence = evidence_path.read_bytes()
            error_output = StringIO()
            with redirect_stderr(error_output):
                exit_code = main([
                    str(assessment_path),
                    "--artifact-root", str(root),
                    "--report", str(evidence_path),
                ])
            self.assertEqual(exit_code, 2)
            self.assertIn("would overwrite the assessment", error_output.getvalue())
            self.assertEqual(evidence_path.read_bytes(), original_evidence)

    @unittest.skipIf(jsonschema is None, "jsonschema dependency is installed by the CI workflow")
    def test_unknown_fixture_conforms_to_pinned_draft_2020_12_schema(self) -> None:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        jsonschema.Draft202012Validator.check_schema(schema)
        jsonschema.Draft202012Validator(
            schema, format_checker=jsonschema.FormatChecker()
        ).validate(assessment())

    @unittest.skipIf(jsonschema is None, "jsonschema dependency is installed by the CI workflow")
    def test_preregistration_artifact_schema_accepts_bound_record(self) -> None:
        doc = assessment()
        constraint = doc["constraints"][0]
        payload = json.loads(preregistration_payload(constraint))
        schema = json.loads(PREREG_SCHEMA_PATH.read_text(encoding="utf-8"))
        jsonschema.Draft202012Validator.check_schema(schema)
        validator = jsonschema.Draft202012Validator(
            schema, format_checker=jsonschema.FormatChecker()
        )
        self.assertEqual(list(validator.iter_errors(payload)), [])

    @unittest.skipIf(jsonschema is None, "jsonschema dependency is installed by the CI workflow")
    def test_subgroup_uncertainty_reference_resolves(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        outcome = doc["alternatives"][0]["outcomes"][1]
        outcome["subgroups"] = [{
            "group_id": "area-a",
            "status": "KNOWN",
            "value": 0.5,
            "uncertainty": {
                "status": "QUANTIFIED",
                "lower": 0.2,
                "upper": 0.8,
                "interpretation": "coverage_interval",
                "coverage_probability": 0.95,
                "method": "synthetic interval",
            },
            "missing_reason": None,
            "artifact_ids": ["sample-artifact"],
        }]
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        validator = jsonschema.Draft202012Validator(
            schema, format_checker=jsonschema.FormatChecker()
        )
        self.assertEqual(list(validator.iter_errors(doc)), [])

    @unittest.skipIf(jsonschema is None, "jsonschema dependency is installed by the CI workflow")
    def test_coverage_interval_requires_probability(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        uncertainty = doc["alternatives"][0]["outcomes"][0]["uncertainty"]
        uncertainty["interpretation"] = "coverage_interval"
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        validator = jsonschema.Draft202012Validator(
            schema, format_checker=jsonschema.FormatChecker()
        )
        self.assertTrue(list(validator.iter_errors(doc)))

    @unittest.skipIf(jsonschema is None, "jsonschema dependency is installed by the CI workflow")
    def test_schema_rejects_unknown_artifact_role(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        doc["artifacts"][0]["role"] = "trusted-source-ish"
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        validator = jsonschema.Draft202012Validator(
            schema, format_checker=jsonschema.FormatChecker()
        )
        self.assertTrue(list(validator.iter_errors(doc)))

    def test_schema_rejects_unknown_value_inconsistency(self) -> None:
        doc = assessment()
        doc["alternatives"][0]["outcomes"][0]["value"] = 0.0
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        validator = jsonschema.Draft202012Validator(
            schema, format_checker=jsonschema.FormatChecker()
        )
        self.assertTrue(list(validator.iter_errors(doc)))


if __name__ == "__main__":
    unittest.main()
