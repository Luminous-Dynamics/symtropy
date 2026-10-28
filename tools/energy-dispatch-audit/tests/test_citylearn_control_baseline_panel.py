import importlib.util
import json
import math
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace

SCRIPT_DIR = Path(__file__).resolve().parents[1] / "scripts"
sys.path.insert(0, str(SCRIPT_DIR))
import citylearn_control_baseline_panel as panel  # noqa: E402


PROTOCOL_PATH = (
    Path(__file__).resolve().parents[1]
    / "fixtures"
    / "energy-citylearn-control-benchmark-protocol-v0.1.json"
)


class CityLearnControlProtocolTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.protocol = json.loads(PROTOCOL_PATH.read_text(encoding="utf-8"))

    def test_frozen_protocol_is_valid(self):
        panel.validate_protocol(self.protocol)

    def test_three_splits_cover_all_steps_without_overlap(self):
        split_rows = self.protocol["splits"]
        self.assertEqual(sum(item["steps"] for item in split_rows), 8760)
        self.assertEqual(split_rows[0]["start_inclusive"], 0)
        self.assertEqual(split_rows[-1]["end_inclusive"], 8759)
        for previous, current in zip(split_rows, split_rows[1:]):
            self.assertEqual(previous["end_inclusive"] + 1, current["start_inclusive"])

    def test_rejects_reordered_or_overlapping_windows(self):
        altered = json.loads(json.dumps(self.protocol))
        altered["splits"][1]["start_inclusive"] += 1
        with self.assertRaisesRegex(ValueError, "split order, bounds|overlap"):
            panel.validate_protocol(altered)

    def test_rejects_wrong_source_commit_or_dataset(self):
        altered = json.loads(json.dumps(self.protocol))
        altered["source"]["commit"] = "0" * 40
        with self.assertRaisesRegex(ValueError, "source pin"):
            panel.validate_protocol(altered)

    def test_rejects_prediction_inputs_on_a_fixed_baseline(self):
        altered = json.loads(json.dumps(self.protocol))
        altered["controllers"][2]["uses_prediction_features"] = True
        with self.assertRaisesRegex(ValueError, "learned/prediction-input"):
            panel.validate_protocol(altered)

    def test_rejects_trained_controllers_in_the_baseline_panel(self):
        altered = json.loads(json.dumps(self.protocol))
        altered["controllers"][1]["learned"] = True
        with self.assertRaisesRegex(ValueError, "learned/prediction-input"):
            panel.validate_protocol(altered)

    def test_json_normalizer_rejects_nonfinite_values_by_representing_them_as_null(self):
        self.assertIsNone(panel.normalize_json(float("nan")))
        self.assertIsNone(panel.normalize_json(float("inf")))
        encoded = panel.canonical_json_bytes({"nan": panel.normalize_json(float("nan"))})
        self.assertEqual(encoded, b'{"nan":null}')

    def test_percentile_uses_nearest_rank_and_empty_is_null(self):
        self.assertEqual(panel.percentile([4.0, 1.0, 2.0, 3.0], 0.50), 2.0)
        self.assertEqual(panel.percentile([4.0, 1.0, 2.0, 3.0], 0.95), 4.0)
        self.assertIsNone(panel.percentile([], 0.99))

    def test_simulation_fails_closed_without_a_rights_review_acknowledgment(self):
        args = SimpleNamespace(
            acknowledge_data_terms_review=False,
            data_terms_review_reference="",
        )
        with self.assertRaisesRegex(ValueError, "blocked until dataset data terms are reviewed"):
            panel.run_panel(args)

    def test_simulation_requires_a_specific_review_reference(self):
        args = SimpleNamespace(
            acknowledge_data_terms_review=True,
            data_terms_review_reference="   ",
        )
        with self.assertRaisesRegex(ValueError, "blocked until dataset data terms are reviewed"):
            panel.run_panel(args)

    def test_adapter_does_not_import_citylearn_at_module_load(self):
        # CityLearn itself is imported only after terms, source and protocol gates pass.
        self.assertNotIn("citylearn.citylearn", sys.modules)

    def test_required_baselines_remain_official_and_ordered(self):
        self.assertEqual(
            [entry["python_class"] for entry in self.protocol["controllers"]],
            [
                "citylearn.agents.base.BaselineAgent",
                "citylearn.agents.baseline.BusinessAsUsualAgent",
                "citylearn.agents.rbc.BasicRBC",
                "citylearn.agents.rbc.OptimizedRBC",
            ],
        )


if __name__ == "__main__":
    unittest.main()
