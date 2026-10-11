import hashlib
import json
import unittest

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
                "classification": "simulated_scenario", "source_model": "mycelix-multiworld-sim",
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
        self.assertEqual(run["artifacts"][0]["artifact_sha256"], hashlib.sha256(raw).hexdigest())
        self.assertEqual(run["artifacts"][0]["artifact_ref"], f"urn:sha256:{hashlib.sha256(raw).hexdigest()}")
        self.assertEqual(len(run["outputs"]), 3)
        self.assertEqual(run["validation"]["evidence"], [])

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

    def test_rejects_ticks_not_matching_year_horizon(self):
        source = self.source()
        source["ticks"] = 25
        source["run_manifest"]["ticks"] = 25
        with self.assertRaisesRegex(AdapterError, r"years \* ticks_per_year"):
            adapt_source(json.dumps(source).encode())


if __name__ == "__main__":
    unittest.main()
