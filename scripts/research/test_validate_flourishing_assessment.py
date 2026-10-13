"""Regression tests for fail-closed Humanity Atlas flourishing assessments."""
from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path

try:
    import jsonschema
except ImportError:  # CI installs the pinned schema validator.
    jsonschema = None

ROOT = Path(__file__).resolve().parents[2]
SCHEMA_PATH = ROOT / "docs/humanity-atlas/flourishing-assessment-v0.1.schema.json"
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


def add_evidence_and_known_values(doc: dict, values: dict[str, float] | None = None) -> dict:
    raw = b"synthetic evidence; not a real observation\n"
    preregistration_bytes = b"preregistered synthetic test thresholds\n"
    doc["artifacts"] = [
        {
            "artifact_id": "sample-artifact",
            "path": "sample.txt",
            "sha256": hashlib.sha256(raw).hexdigest(),
            "purpose": "synthetic verifier test input",
            "collected_at": "2026-10-13T10:30:00Z",
        },
        {
            "artifact_id": "preregistration-artifact",
            "path": "preregistration.json",
            "sha256": hashlib.sha256(preregistration_bytes).hexdigest(),
            "purpose": "pre-registration",
            "collected_at": "2026-10-13T09:30:00Z",
        },
    ]
    for constraint in doc["constraints"]:
        constraint["preregistration_artifact_id"] = "preregistration-artifact"
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

    def test_known_passing_constraints_produce_comparison_readiness_not_a_ranking(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        errors, report = validate_assessment(doc)
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
        errors, report = validate_assessment(doc)
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
        errors, report = validate_assessment(doc)
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
        errors, report = validate_assessment(doc)
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
        errors, report = validate_assessment(doc)
        self.assertEqual(errors, [])
        by_id = {a["alternative_id"]: a for a in report["alternatives"]}
        self.assertEqual(by_id["candidate"]["constraint_results"][0]["status"], "UNKNOWN")
        self.assertEqual(by_id["candidate"]["comparison_status"], "UNRESOLVED")

    def test_unquantified_non_constraint_outcome_is_incomplete(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        for alternative in doc["alternatives"]:
            alternative["outcomes"][0]["uncertainty"] = {
                "status": "NOT_QUANTIFIED",
                "method": "uncertainty budget assessment",
                "reason": "no defensible interval is available",
            }
        errors, report = validate_assessment(doc)
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

    def test_artifact_digest_is_checked_against_file_bytes(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "sample.txt").write_bytes(b"bytes differ from the declared digest")
            (root / "preregistration.json").write_bytes(b"preregistered synthetic test thresholds\n")
            errors, _ = validate_assessment(doc, artifact_root=root, verify_artifacts=True)
        self.assertTrue(any("SHA-256 mismatch" in error for error in errors))

    def test_subgroup_violation_is_not_hidden_by_an_unknown_subgroup(self) -> None:
        doc = add_evidence_and_known_values(assessment())
        constraint = doc["constraints"][0]
        constraint["scope"] = "overall_and_subgroups"
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
        errors, report = validate_assessment(doc)
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
        self.assertIsNone(report["ranking"])
        self.assertEqual(report["assessment_status"], "VALIDATED_ASSESSMENT_NOT_POLICY_APPROVAL")
        self.assertEqual(
            {item["comparison_status"] for item in report["alternatives"]},
            {"UNRESOLVED"},
        )

    @unittest.skipIf(jsonschema is None, "jsonschema dependency is installed by the CI workflow")
    def test_unknown_fixture_conforms_to_pinned_draft_2020_12_schema(self) -> None:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        jsonschema.Draft202012Validator.check_schema(schema)
        jsonschema.Draft202012Validator(
            schema, format_checker=jsonschema.FormatChecker()
        ).validate(assessment())

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
