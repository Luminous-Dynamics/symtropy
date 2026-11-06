import csv
import io
import json
import unittest
from pathlib import Path

import jsonschema

from adapt_civbench_output import (
    AdapterError,
    KNOWLEDGE_HEADER,
    RUN_SCHEMA_VERSION,
    SERVICE_HEADER,
    _model_manifest,
    build_envelope,
    canonical_json,
    sha256,
    strict_json,
)

HERE = Path(__file__).resolve().parent
SCHEMA_DIR = HERE / "schemas"
MODEL_PATH = Path("docs/ops/humanity_atlas_scenario_contract_v0_1/fixtures/civbench-test-model.rs")


def fixture_scenario(track: str, dataset_bytes: bytes, model_sha: str) -> bytes:
    scenario_id = f"civbench.{track}.synthetic.v0.1"
    if track == "service-cascade":
        horizon_end = 5
        metrics = [
            {"metric_id": "service.hospital.effective_service", "unit": "fraction", "objective": "report_only"},
            {"metric_id": "scenario.demand_weighted_service_deficit_area", "unit": "demand_weighted_service_ticks", "objective": "report_only"},
        ]
        intervention = {
            "target_id": "grid",
            "parameter": "own_availability",
            "operation": "set",
            "value": 0.0,
            "unit": "fraction",
            "effective_year": 1,
            "expires_year": 3,
        }
        dataset_id = "synthetic-service-cascade-fixture-v0.1"
    else:
        horizon_end = 0
        metrics = [
            {"metric_id": "knowledge.hydrology_skill.acquisition_count", "unit": "count", "objective": "report_only"},
            {"metric_id": "knowledge.silt_myth.refuted_knowledge_acquisition_count", "unit": "count", "objective": "report_only"},
        ]
        intervention = {
            "target_id": "synthetic_knowledge_network",
            "parameter": "transmission_policy",
            "operation": "set",
            "value": 1.0,
            "unit": "fraction",
            "effective_year": 0,
        }
        dataset_id = "synthetic-knowledge-resilience-fixture-v0.1"

    document = {
        "schema_version": "humanity-atlas.scenario.v0.1",
        "scenario_id": scenario_id,
        "title": f"Synthetic {track} reference fixture",
        "intent": "design_study",
        "baseline": {
            "dataset_id": dataset_id,
            "dataset_sha256": sha256(dataset_bytes),
            "reference_year": 0,
        },
        "horizon": {"start_year": 0, "end_year": horizon_end, "step_years": 1},
        "engine": {
            "adapter": f"civdyn-{track}",
            "engine_version": "0.1.0",
            "model_sha256": model_sha,
            "deterministic": True,
            "seed": 0,
            "replicates": 1,
        },
        "interventions": [intervention],
        "assumptions": [
            {
                "id": "synthetic_fixture_only",
                "kind": "fixed",
                "statement": "All inputs are synthetic software fixtures.",
                "value": "synthetic",
                "unit": "description",
            }
        ],
        "requested_metrics": metrics,
        "limitations": [
            "This scenario is a software fixture, not an empirical observation or forecast."
        ],
    }
    return canonical_json(document)


def csv_bytes(header: list[str], rows: list[list[str]]) -> bytes:
    stream = io.StringIO(newline="")
    writer = csv.writer(stream, lineterminator="\n")
    writer.writerow(header)
    writer.writerows(rows)
    return stream.getvalue().encode("utf-8")


class CivBenchAtlasAdapterTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.run_schema = strict_json(
            (SCHEMA_DIR / "run-v0.1.schema.json").read_bytes(),
            "run-v0.1 schema",
        )
        cls.scenario_schema = strict_json(
            (SCHEMA_DIR / "scenario-v0.1.schema.json").read_bytes(),
            "scenario-v0.1 schema",
        )
        cls.model_sha, cls.model_manifest = _model_manifest([MODEL_PATH])

    def dataset(self, track: str) -> bytes:
        return canonical_json(
            {
                "dataset_id": f"synthetic-{track}",
                "record_type": "synthetic_fixture_manifest",
                "empirical_observations": False,
                "source_note": "Generated test fixture; no human or utility data.",
            }
        )

    def service_csv(self, scenario_id: str) -> bytes:
        rows = [
            ["tick", scenario_id, "hospital", "healthcare", "0", "1", "1", "", "2", "", "", "", "", "", "", "", ""],
            ["tick", scenario_id, "hospital", "healthcare", "1", "0", "0", "", "2", "", "", "", "", "", "", "", ""],
            ["node_summary", scenario_id, "hospital", "healthcare", "", "", "", "0", "2", "1", "1", "1", "2", "", "", "", ""],
            ["criticality", scenario_id, "grid", "power", "", "", "", "", "", "", "", "", "", "0", "16", "16", ""],
            ["scenario_summary", scenario_id, "", "", "", "", "", "", "8", "", "", "", "", "", "", "", "16"],
        ]
        return csv_bytes(SERVICE_HEADER, rows)

    def knowledge_csv(self) -> bytes:
        rows = [
            [
                "item_summary", "hydrology_skill", "aggregate", "", "verified", "water_systems",
                "2", "4", "3", "3", "3", "3", "3", "3", "3", "2", "2", "0", "3", "3", "3", "2", "", "",
            ],
            [
                "item_summary", "silt_myth", "aggregate", "", "refuted", "water_systems",
                "1", "2", "2", "2", "1", "1", "1", "1", "1", "1", "1", "1", "0", "0", "0", "0", "", "",
            ],
            [
                "item_summary", "silt_myth", "aggregate", "", "refuted", "water_systems",
                "1", "2", "2", "2", "1", "1", "1", "1", "1", "1", "1", "1", "0", "0", "0", "0", "", "",
            ],
            [
                "transmission_event", "hydrology_skill", "demonstration", "agent_a->agent_c",
                "", "", "", "", "", "true", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
            ],
            [
                "archive_recovery", "hydrology_skill", "archive_domain_a_failed", "",
                "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "2", "true",
            ],
        ]
        return csv_bytes(KNOWLEDGE_HEADER, rows)

    def adapt(self, track: str, csv_data: bytes, scenario_data: bytes, dataset_data: bytes, replay: bytes | None = None):
        from adapt_civbench_output import build_envelope

        return build_envelope(
            track=track,
            csv_bytes=csv_data,
            csv_ref=f"urn:sha256:{sha256(csv_data)}",
            scenario_bytes=scenario_data,
            scenario_ref=f"urn:sha256:{sha256(scenario_data)}",
            dataset_bytes=dataset_data,
            dataset_ref=f"urn:sha256:{sha256(dataset_data)}",
            model_sha=self.model_sha,
            model_manifest=self.model_manifest,
            model_manifest_ref=f"urn:sha256:{sha256(self.model_manifest)}",
            source_revision="a" * 40,
            replay_bytes=replay,
        )

    def validate_run(self, envelope: dict) -> None:
        jsonschema.Draft202012Validator(self.run_schema).validate(envelope)

    def test_service_cascade_maps_native_units_and_retains_complete_csv(self) -> None:
        dataset = self.dataset("service-cascade")
        scenario = fixture_scenario("service-cascade", dataset, self.model_sha)
        scenario_id = json.loads(scenario)["scenario_id"]
        report = self.service_csv(scenario_id)
        envelope = self.adapt("service-cascade", report, scenario, dataset)
        self.validate_run(envelope)

        self.assertEqual(envelope["output_classification"], "synthetic_fixture")
        self.assertEqual(envelope["validation"]["status"], "contract_smoke")
        self.assertIsNone(envelope["deterministic"])
        self.assertEqual(envelope["inputs"]["dataset_sha256"], sha256(dataset))
        self.assertEqual(envelope["artifacts"][0]["artifact_sha256"], sha256(report))

        metric_map = {item["metric_id"]: item for item in envelope["outputs"]}
        self.assertEqual(metric_map["service.hospital.effective_service"]["unit"], "fraction")
        self.assertEqual(metric_map["service.hospital.effective_service"]["year"], 1)
        self.assertEqual(metric_map["scenario.demand_weighted_service_deficit_area"]["value"], 16.0)
        self.assertEqual(metric_map["scenario.demand_weighted_service_deficit_area"]["unit"], "demand_weighted_service_ticks")

    def test_knowledge_resilience_preserves_epistemic_labels_in_source_artifact(self) -> None:
        dataset = self.dataset("knowledge-resilience")
        scenario = fixture_scenario("knowledge-resilience", dataset, self.model_sha)
        report = self.knowledge_csv()
        envelope = self.adapt("knowledge-resilience", report, scenario, dataset)
        self.validate_run(envelope)

        metrics = {item["metric_id"]: item for item in envelope["outputs"]}
        self.assertEqual(metrics["knowledge.hydrology_skill.acquisition_count"]["value"], 3.0)
        self.assertEqual(metrics["knowledge.silt_myth.refuted_knowledge_acquisition_count"]["value"], 1.0)
        self.assertIn(b",verified,", report)
        self.assertIn(b",refuted,", report)
        self.assertEqual(envelope["artifacts"][0]["artifact_sha256"], sha256(report))

    def test_byte_identical_replay_is_the_only_path_to_deterministic_true(self) -> None:
        dataset = self.dataset("service-cascade")
        scenario = fixture_scenario("service-cascade", dataset, self.model_sha)
        scenario_id = json.loads(scenario)["scenario_id"]
        report = self.service_csv(scenario_id)

        unknown = self.adapt("service-cascade", report, scenario, dataset)
        repeated = self.adapt("service-cascade", report, scenario, dataset, replay=report)
        changed = self.adapt("service-cascade", report, scenario, dataset, replay=report + b"\n")

        self.assertIsNone(unknown["deterministic"])
        self.assertTrue(repeated["deterministic"])
        self.assertEqual(len(repeated["validation"]["evidence"]), 1)
        self.assertFalse(changed["deterministic"])
        self.assertEqual(changed["validation"]["evidence"], [])

    def test_declared_nondeterministic_engine_stays_false_even_if_one_replay_matches(self) -> None:
        dataset = self.dataset("service-cascade")
        scenario_doc = json.loads(fixture_scenario("service-cascade", dataset, self.model_sha))
        scenario_doc["engine"]["deterministic"] = False
        scenario = canonical_json(scenario_doc)
        report = self.service_csv(scenario_doc["scenario_id"])
        envelope = self.adapt("service-cascade", report, scenario, dataset, replay=report)
        self.validate_run(envelope)
        self.assertFalse(envelope["deterministic"])
        self.assertEqual(envelope["validation"]["evidence"], [])

    def test_maximum_length_scenario_id_produces_schema_valid_run_id(self) -> None:
        dataset = self.dataset("service-cascade")
        scenario_doc = json.loads(fixture_scenario("service-cascade", dataset, self.model_sha))
        scenario_doc["scenario_id"] = "a" * 180
        scenario = canonical_json(scenario_doc)
        report = self.service_csv(scenario_doc["scenario_id"])
        envelope = self.adapt("service-cascade", report, scenario, dataset)
        self.validate_run(envelope)
        self.assertLessEqual(len(envelope["run_id"]), 200)

    def test_dataset_digest_mismatch_fails_closed(self) -> None:
        dataset = self.dataset("service-cascade")
        scenario_doc = json.loads(fixture_scenario("service-cascade", dataset, self.model_sha))
        scenario_doc["baseline"]["dataset_sha256"] = "0" * 64
        scenario = canonical_json(scenario_doc)
        scenario_id = scenario_doc["scenario_id"]
        with self.assertRaises(AdapterError):
            self.adapt("service-cascade", self.service_csv(scenario_id), scenario, dataset)

    def test_duplicate_csv_headers_fail_closed(self) -> None:
        dataset = self.dataset("service-cascade")
        scenario = fixture_scenario("service-cascade", dataset, self.model_sha)
        with self.assertRaises(AdapterError):
            self.adapt(
                "service-cascade",
                b"record_type,record_type\nx,x\n",
                scenario,
                dataset,
            )

    def test_non_finite_numeric_metrics_fail_closed(self) -> None:
        dataset = self.dataset("service-cascade")
        scenario = fixture_scenario("service-cascade", dataset, self.model_sha)
        scenario_id = json.loads(scenario)["scenario_id"]
        report = self.service_csv(scenario_id)
        malformed = report.replace(b",0,1,1,", b",0,nan,1,", 1)
        self.assertNotEqual(report, malformed)
        with self.assertRaises(AdapterError):
            self.adapt("service-cascade", malformed, scenario, dataset)

    def test_calibration_promotion_without_receipt_is_rejected_by_run_schema(self) -> None:
        dataset = self.dataset("service-cascade")
        scenario = fixture_scenario("service-cascade", dataset, self.model_sha)
        scenario_id = json.loads(scenario)["scenario_id"]
        envelope = self.adapt(
            "service-cascade",
            self.service_csv(scenario_id),
            scenario,
            dataset,
        )
        envelope["output_classification"] = "calibrated_simulation"
        with self.assertRaises(jsonschema.ValidationError):
            self.validate_run(envelope)

    def test_scenario_model_digest_must_match_declared_model_source_snapshot(self) -> None:
        dataset = self.dataset("knowledge-resilience")
        scenario_doc = json.loads(fixture_scenario("knowledge-resilience", dataset, self.model_sha))
        scenario_doc["engine"]["model_sha256"] = "f" * 64
        scenario = canonical_json(scenario_doc)
        with self.assertRaises(AdapterError):
            self.adapt("knowledge-resilience", self.knowledge_csv(), scenario, dataset)


if __name__ == "__main__":
    unittest.main()
