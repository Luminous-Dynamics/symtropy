import hashlib
import json
import unittest
from pathlib import Path

from adapt_multiworld_output import AdapterError, adapt_source, canonical_json_bytes


class AdapterTests(unittest.TestCase):
    def source(self):
        cfg = {"years": 2, "policy": {"rate": 0.5}}
        params = {"decay": 0.25}
        cfg_sha = hashlib.sha256(canonical_json_bytes(cfg)).hexdigest()
        params_sha = hashlib.sha256(canonical_json_bytes(params)).hexdigest()
        model_sha = "a" * 64
        return {
            "seed": 42, "years": 2, "ticks": 24, "survived": True,
            "final_cvs": 0.73, "final_population": 1234, "tech_milestones": 4,
            "resolved_config": cfg, "resolved_params": params,
            "technology_tree": {
                "schema_version": "mycelix-multiworld-sim.tech-tree.v1",
                "classification": "simulated_scenario", "source_model": "mycelix-multiworld-sim/0.1.0",
                "definition_sha256": model_sha, "definition_source": "in_code_default_tree",
                "ticks_per_year": 12, "year_zero_semantics": "relative_to_scenario_start",
                "milestones": [{"id": "synthetic-only"}],
            },
            "run_manifest": {
                "schema_version": "humanity-atlas.run-manifest.v1",
                "selected_definition_source": "in_code_default_tree", "effective_model_sha256": model_sha,
                "resolved_config_sha256": cfg_sha, "resolved_params_sha256": params_sha,
                "seed": 42, "ticks": 24, "source_revision": None,
            },
        }

    def raw_source(self):
        return json.dumps(self.source(), separators=(",", ":"), allow_nan=False).encode()

    def test_emits_conservative_bound_envelope(self):
        raw = self.raw_source()
        run = adapt_source(raw)
        self.assertEqual(run["run_status"], "completed")
        self.assertEqual(run["output_classification"], "unvalidated_simulation")
        self.assertIsNone(run["deterministic"])
        self.assertIsNone(run["inputs"]["dataset_sha256"])
        self.assertEqual(run["engine"]["model_identity_kind"], "technology_tree_definition")
        self.assertEqual(run["engine"]["engine_version"], "0.1.0")
        self.assertIsNone(run["engine"]["source_revision"])
        self.assertEqual(run["inputs"]["scenario_identity_kind"], "canonical_effective_scenario")
        self.assertEqual(run["inputs"]["selected_definition_source"], "in_code_default_tree")
        self.assertTrue(run["scenario_id"].endswith(run["inputs"]["scenario_sha256"]))
        self.assertEqual(run["artifacts"][0]["artifact_sha256"], hashlib.sha256(raw).hexdigest())
        self.assertEqual(run["artifacts"][0]["artifact_ref"], f"urn:sha256:{hashlib.sha256(raw).hexdigest()}")
        self.assertEqual(run["artifacts"][0]["json_pointer"], "/technology_tree")
        self.assertEqual(len(run["outputs"]), 3)
        self.assertTrue(all(row["time_basis"] == "scenario_relative_year" for row in run["outputs"]))
        self.assertEqual(run["validation"]["evidence"], [])


    def test_generated_envelope_matches_run_schema_when_jsonschema_available(self):
        try:
            import jsonschema
        except ImportError:
            self.skipTest("jsonschema is optional in the local process environment")
        schema_path = Path(__file__).parent / "schemas" / "run-v0.1.schema.json"
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
        jsonschema.Draft202012Validator.check_schema(schema)
        jsonschema.validate(instance=adapt_source(self.raw_source()), schema=schema)

    def test_rejects_tick_counts_outside_u32_wire_range(self):
        for field in ("ticks", "run_manifest_ticks", "ticks_per_year"):
            source = self.source()
            if field == "ticks":
                source["ticks"] = 4_294_967_296
                source["run_manifest"]["ticks"] = 4_294_967_296
            elif field == "run_manifest_ticks":
                source["run_manifest"]["ticks"] = 4_294_967_296
            else:
                source["technology_tree"]["ticks_per_year"] = 4_294_967_296
            with self.subTest(field=field):
                with self.assertRaisesRegex(AdapterError, "u32 wire range"):
                    adapt_source(json.dumps(source).encode())

    def test_rejects_source_revision_above_schema_limit(self):
        source = self.source()
        source["run_manifest"]["source_revision"] = "r" * 121
        with self.assertRaisesRegex(AdapterError, "at most 120 characters"):
            adapt_source(json.dumps(source).encode())

    def test_rejects_invalid_package_version_token(self):
        source = self.source()
        source["technology_tree"]["source_model"] = "mycelix-multiworld-sim/0.1.0/extra"
        with self.assertRaisesRegex(AdapterError, "version token"):
            adapt_source(json.dumps(source).encode())

    def test_rejects_overlong_artifact_reference(self):
        with self.assertRaisesRegex(AdapterError, "at most 2000 characters"):
            adapt_source(self.raw_source(), artifact_ref="x" * 2001)

    def test_rejects_missing_engine_package_version(self):
        source = self.source()
        source["technology_tree"]["source_model"] = "mycelix-multiworld-sim"
        with self.assertRaisesRegex(AdapterError, "versioned mycelix-multiworld-sim engine"):
            adapt_source(json.dumps(source).encode())

    def test_rejects_manifest_model_mismatch(self):
        source = self.source()
        source["run_manifest"]["effective_model_sha256"] = "b" * 64
        with self.assertRaisesRegex(AdapterError, "disagrees with run manifest"):
            adapt_source(json.dumps(source).encode())

    def test_rejects_input_digest_mismatch(self):
        source = self.source()
        source["resolved_config"]["years"] = 3
        with self.assertRaisesRegex(AdapterError, "resolved_config does not match"):
            adapt_source(json.dumps(source).encode())

    def test_rejects_duplicate_json_key(self):
        with self.assertRaisesRegex(AdapterError, "duplicate JSON object key"):
            adapt_source(b'{"seed":1,"seed":2}')

    def test_rejects_nan(self):
        with self.assertRaisesRegex(AdapterError, "non-standard JSON numeric token"):
            adapt_source(b'{"value":NaN}')

    def test_rejects_seed_tick_manifest_mismatch(self):
        source = self.source()
        source["seed"] = 7
        with self.assertRaisesRegex(AdapterError, "disagree with run manifest"):
            adapt_source(json.dumps(source).encode())

    def test_rejects_horizon_outside_metric_schema_range(self):
        source = self.source()
        source["years"] = 100001
        source["ticks"] = 1200012
        source["run_manifest"]["ticks"] = 1200012
        source["technology_tree"]["ticks_per_year"] = 12
        with self.assertRaisesRegex(AdapterError, "year <= 100000"):
            adapt_source(json.dumps(source).encode())

    def test_rejects_ticks_not_matching_year_horizon(self):
        source = self.source()
        source["ticks"] = 25
        source["run_manifest"]["ticks"] = 25
        with self.assertRaisesRegex(AdapterError, r"years \* ticks_per_year"):
            adapt_source(json.dumps(source).encode())


if __name__ == "__main__":
    unittest.main()
