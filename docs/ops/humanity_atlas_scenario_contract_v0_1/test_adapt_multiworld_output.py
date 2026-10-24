import hashlib
import json
import os
import tempfile
import unittest
from contextlib import redirect_stderr
from io import BytesIO, StringIO
from types import SimpleNamespace
from unittest.mock import patch
from pathlib import Path

from adapt_multiworld_output import (
    AdapterError, MAX_SOURCE_ARTIFACT_BYTES, _check_source_artifact_size,
    adapt_source, canonical_json_bytes, main, read_source_artifact,
)


class AdapterTests(unittest.TestCase):
    def source(self):
        cfg = {"seed": 42, "total_ticks": 24, "policy": {"rate": 0.5}}
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
        self.assertEqual(len(run["outputs"]), 4)
        self.assertEqual(
            next(row for row in run["outputs"] if row["metric_id"] == "civilization_survived")["value"],
            1.0,
        )
        self.assertTrue(all(row["time_basis"] == "scenario_relative_year" for row in run["outputs"]))
        self.assertEqual(run["validation"]["evidence"], [])


    def test_survival_failure_is_preserved_as_terminal_output_and_warning(self):
        source = self.source()
        source["survived"] = False
        source["final_population"] = 0
        raw = json.dumps(source, separators=(",", ":"), allow_nan=False).encode()
        run = adapt_source(raw)
        survival = next(row for row in run["outputs"] if row["metric_id"] == "civilization_survived")
        self.assertEqual(survival["value"], 0.0)
        self.assertEqual(survival["year"], source["years"])
        self.assertEqual(run["run_status"], "completed")
        self.assertTrue(any("scenario_survival_failure" in warning for warning in run["warnings"]))

    def test_generated_envelope_matches_run_schema_when_jsonschema_available(self):
        try:
            import jsonschema
        except ImportError:
            if os.environ.get("HUMANITY_ATLAS_REQUIRE_JSONSCHEMA") == "1":
                self.fail("jsonschema is required by the end-to-end qualification gate")
            self.skipTest("jsonschema is optional in an ordinary local regression run")
        schema_path = Path(__file__).parent / "schemas" / "run-v0.1.schema.json"
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
        jsonschema.Draft202012Validator.check_schema(schema)
        jsonschema.validate(instance=adapt_source(self.raw_source()), schema=schema)

    def test_cli_refuses_to_overwrite_the_captured_source_artifact(self):
        raw = self.raw_source()
        with tempfile.TemporaryDirectory() as temp:
            source_path = Path(temp) / "simulator-output.json"
            source_path.write_bytes(raw)
            stderr = StringIO()
            with redirect_stderr(stderr):
                status = main([str(source_path), "--output", str(source_path)])
            self.assertEqual(status, 2)
            self.assertIn("refusing to overwrite captured source bytes", stderr.getvalue())
            self.assertEqual(source_path.read_bytes(), raw)

    def test_cli_refuses_to_overwrite_an_existing_output_without_flag(self):
        raw = self.raw_source()
        preserved = b"previous evidence must remain unchanged\n"
        with tempfile.TemporaryDirectory() as temp:
            source_path = Path(temp) / "simulator-output.json"
            output_path = Path(temp) / "run-envelope.json"
            source_path.write_bytes(raw)
            output_path.write_bytes(preserved)
            stderr = StringIO()
            with redirect_stderr(stderr):
                status = main([str(source_path), "--output", str(output_path)])
            self.assertEqual(status, 2)
            self.assertIn("refusing to replace it without --overwrite", stderr.getvalue())
            self.assertEqual(output_path.read_bytes(), preserved)

    def test_cli_overwrite_replaces_only_selected_hard_link_entry(self):
        raw = self.raw_source()
        preserved = b"previous evidence must remain unchanged\n"
        with tempfile.TemporaryDirectory() as temp:
            source_path = Path(temp) / "simulator-output.json"
            output_path = Path(temp) / "run-envelope.json"
            alias_path = Path(temp) / "second-reference-to-old-output.json"
            source_path.write_bytes(raw)
            output_path.write_bytes(preserved)
            try:
                os.link(output_path, alias_path)
            except (OSError, NotImplementedError) as exc:
                self.skipTest(f"hard-link creation is unavailable on this filesystem: {exc}")
            status = main([
                str(source_path), "--output", str(output_path), "--overwrite"
            ])
            self.assertEqual(status, 0)
            envelope = json.loads(output_path.read_text(encoding="utf-8"))
            self.assertEqual(envelope["run_id"], f"urn:sha256:{hashlib.sha256(raw).hexdigest()}")
            self.assertEqual(alias_path.read_bytes(), preserved)
            self.assertEqual(source_path.read_bytes(), raw)

    def test_cli_overwrite_rejects_source_hard_link_alias(self):
        raw = self.raw_source()
        with tempfile.TemporaryDirectory() as temp:
            source_path = Path(temp) / "simulator-output.json"
            output_alias = Path(temp) / "output-alias.json"
            source_path.write_bytes(raw)
            try:
                os.link(source_path, output_alias)
            except (OSError, NotImplementedError) as exc:
                self.skipTest(f"hard-link creation is unavailable on this filesystem: {exc}")
            stderr = StringIO()
            with redirect_stderr(stderr):
                status = main([
                    str(source_path), "--output", str(output_alias), "--overwrite"
                ])
            self.assertEqual(status, 2)
            self.assertIn("refusing to overwrite captured source bytes", stderr.getvalue())
            self.assertEqual(source_path.read_bytes(), raw)
            self.assertEqual(output_alias.read_bytes(), raw)

    def test_cli_overwrite_replaces_symlink_entry_without_mutating_target(self):
        raw = self.raw_source()
        preserved = b"separate target evidence remains unchanged\n"
        with tempfile.TemporaryDirectory() as temp:
            source_path = Path(temp) / "simulator-output.json"
            output_path = Path(temp) / "run-envelope.json"
            target_path = Path(temp) / "existing-target.json"
            source_path.write_bytes(raw)
            target_path.write_bytes(preserved)
            try:
                output_path.symlink_to(target_path)
            except (OSError, NotImplementedError) as exc:
                self.skipTest(f"symlink creation is unavailable on this filesystem: {exc}")
            status = main([
                str(source_path), "--output", str(output_path), "--overwrite"
            ])
            self.assertEqual(status, 0)
            self.assertFalse(output_path.is_symlink())
            envelope = json.loads(output_path.read_text(encoding="utf-8"))
            self.assertEqual(envelope["run_id"], f"urn:sha256:{hashlib.sha256(raw).hexdigest()}")
            self.assertEqual(target_path.read_bytes(), preserved)
            self.assertEqual(source_path.read_bytes(), raw)

    def test_cli_creates_new_output_when_path_is_absent(self):
        raw = self.raw_source()
        with tempfile.TemporaryDirectory() as temp:
            source_path = Path(temp) / "simulator-output.json"
            output_path = Path(temp) / "run-envelope.json"
            source_path.write_bytes(raw)
            status = main([str(source_path), "--output", str(output_path)])
            self.assertEqual(status, 0)
            envelope = json.loads(output_path.read_text(encoding="utf-8"))
            self.assertEqual(envelope["run_id"], f"urn:sha256:{hashlib.sha256(raw).hexdigest()}")
            self.assertEqual(source_path.read_bytes(), raw)

    def test_cli_rejects_symlink_loop_without_traceback(self):
        raw = self.raw_source()
        with tempfile.TemporaryDirectory() as temp:
            source_path = Path(temp) / "simulator-output.json"
            output_loop = Path(temp) / "output-loop.json"
            source_path.write_bytes(raw)
            try:
                output_loop.symlink_to(output_loop)
            except (OSError, NotImplementedError) as exc:
                self.skipTest(f"symlink creation is unavailable on this filesystem: {exc}")
            stderr = StringIO()
            with redirect_stderr(stderr):
                status = main([str(source_path), "--output", str(output_loop)])
            self.assertEqual(status, 2)
            self.assertIn("cannot safely resolve input/output paths", stderr.getvalue())
            self.assertNotIn("Traceback", stderr.getvalue())
            self.assertEqual(source_path.read_bytes(), raw)

    def test_cli_overwrite_rejects_source_symlink_alias(self):
        raw = self.raw_source()
        with tempfile.TemporaryDirectory() as temp:
            source_path = Path(temp) / "simulator-output.json"
            output_alias = Path(temp) / "output-alias.json"
            source_path.write_bytes(raw)
            try:
                output_alias.symlink_to(source_path)
            except (OSError, NotImplementedError) as exc:
                self.skipTest(f"symlink creation is unavailable on this filesystem: {exc}")
            stderr = StringIO()
            with redirect_stderr(stderr):
                status = main([
                    str(source_path), "--output", str(output_alias), "--overwrite"
                ])
            self.assertEqual(status, 2)
            self.assertIn("refusing to overwrite captured source bytes", stderr.getvalue())
            self.assertTrue(output_alias.is_symlink())
            self.assertEqual(source_path.read_bytes(), raw)

    def test_rejects_non_boolean_survival_value(self):
        source = self.source()
        source["survived"] = "false"
        with self.assertRaisesRegex(AdapterError, "source.survived must be a boolean"):
            adapt_source(json.dumps(source, separators=(",", ":")).encode())

    def test_cli_refuses_to_overwrite_a_hard_link_to_the_source_artifact(self):
        raw = self.raw_source()
        with tempfile.TemporaryDirectory() as temp:
            source_path = Path(temp) / "simulator-output.json"
            output_alias = Path(temp) / "output-alias.json"
            source_path.write_bytes(raw)
            try:
                os.link(source_path, output_alias)
            except (OSError, NotImplementedError) as exc:
                self.skipTest(f"hard-link creation is unavailable on this filesystem: {exc}")
            stderr = StringIO()
            with redirect_stderr(stderr):
                status = main([str(source_path), "--output", str(output_alias)])
            self.assertEqual(status, 2)
            self.assertIn("refusing to overwrite captured source bytes", stderr.getvalue())
            self.assertEqual(source_path.read_bytes(), raw)
            self.assertEqual(output_alias.read_bytes(), raw)

    def test_reader_rejects_growth_after_metadata_check(self):
        fake_path = Path("simulated-artifact.json")
        with (
            patch("adapt_multiworld_output.MAX_SOURCE_ARTIFACT_BYTES", 8),
            patch.object(Path, "stat", return_value=SimpleNamespace(st_size=4)),
            patch.object(Path, "open", return_value=BytesIO(b"x" * 9)),
        ):
            with self.assertRaisesRegex(AdapterError, "source simulator artifact exceeds"):
                read_source_artifact(fake_path)

    def test_source_artifact_size_limit_is_explicit_and_fail_closed(self):
        _check_source_artifact_size(MAX_SOURCE_ARTIFACT_BYTES)
        with self.assertRaisesRegex(AdapterError, "source simulator artifact exceeds"):
            _check_source_artifact_size(MAX_SOURCE_ARTIFACT_BYTES + 1)

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

    def test_rejects_missing_required_source_revision_key(self):
        source = self.source()
        del source["run_manifest"]["source_revision"]
        with self.assertRaisesRegex(AdapterError, "required key and may be null"):
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

    def test_rejects_resolved_config_seed_mismatch_even_with_matching_digest(self):
        source = self.source()
        source["resolved_config"]["seed"] = 7
        source["run_manifest"]["resolved_config_sha256"] = hashlib.sha256(
            canonical_json_bytes(source["resolved_config"])
        ).hexdigest()
        with self.assertRaisesRegex(AdapterError, "resolved_config.seed disagrees"):
            adapt_source(json.dumps(source).encode())

    def test_rejects_resolved_config_tick_mismatch_even_with_matching_digest(self):
        source = self.source()
        source["resolved_config"]["total_ticks"] = 36
        source["run_manifest"]["resolved_config_sha256"] = hashlib.sha256(
            canonical_json_bytes(source["resolved_config"])
        ).hexdigest()
        with self.assertRaisesRegex(AdapterError, "resolved_config.total_ticks disagrees"):
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
        source["resolved_config"]["seed"] = 7
        source["run_manifest"]["resolved_config_sha256"] = hashlib.sha256(
            canonical_json_bytes(source["resolved_config"])
        ).hexdigest()
        with self.assertRaisesRegex(AdapterError, "disagree with run manifest"):
            adapt_source(json.dumps(source).encode())

    def test_rejects_horizon_outside_metric_schema_range(self):
        source = self.source()
        source["years"] = 100001
        source["ticks"] = 1200012
        source["run_manifest"]["ticks"] = 1200012
        source["resolved_config"]["total_ticks"] = 1200012
        source["run_manifest"]["resolved_config_sha256"] = hashlib.sha256(
            canonical_json_bytes(source["resolved_config"])
        ).hexdigest()
        source["technology_tree"]["ticks_per_year"] = 12
        with self.assertRaisesRegex(AdapterError, "year <= 100000"):
            adapt_source(json.dumps(source).encode())

    def test_rejects_ticks_not_matching_year_horizon(self):
        source = self.source()
        source["ticks"] = 25
        source["run_manifest"]["ticks"] = 25
        source["resolved_config"]["total_ticks"] = 25
        source["run_manifest"]["resolved_config_sha256"] = hashlib.sha256(
            canonical_json_bytes(source["resolved_config"])
        ).hexdigest()
        with self.assertRaisesRegex(AdapterError, r"years \* ticks_per_year"):
            adapt_source(json.dumps(source).encode())


if __name__ == "__main__":
    unittest.main()
