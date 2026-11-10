import copy
import json
import unittest
from pathlib import Path

from validate_fixture import validate_fixture


FIXTURE_PATH = (
    Path(__file__).parents[2]
    / "docs"
    / "research"
    / "road_option_experiment_v0_1"
    / "analytic_three_arm_fixture.json"
)


class FixtureSemanticTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        with FIXTURE_PATH.open("r", encoding="utf-8") as stream:
            cls.fixture = json.load(stream)

    def test_committed_fixture_is_valid(self):
        self.assertEqual(validate_fixture(self.fixture), [])

    def test_missing_action_outcome_is_rejected(self):
        mutant = copy.deepcopy(self.fixture)
        mutant["continuation_cases"][0]["outcomes"].pop()
        self.assertTrue(any("exactly one outcome" in e or "do not cover" in e for e in validate_fixture(mutant)))

    def test_duplicate_action_outcome_is_rejected(self):
        mutant = copy.deepcopy(self.fixture)
        mutant["continuation_cases"][0]["outcomes"].append(
            copy.deepcopy(mutant["continuation_cases"][0]["outcomes"][0])
        )
        self.assertTrue(any("duplicate action outcomes" in e for e in validate_fixture(mutant)))

    def test_same_observation_class_cannot_have_two_receipts(self):
        mutant = copy.deepcopy(self.fixture)
        mutant["continuation_cases"][1]["observation_receipt_id"] = "different-receipt"
        self.assertTrue(any("conflicting receipts" in e for e in validate_fixture(mutant)))

    def test_one_receipt_cannot_split_into_two_classes(self):
        mutant = copy.deepcopy(self.fixture)
        mutant["continuation_cases"][2]["observation_class_id"] = "forged-new-class"
        self.assertTrue(any("assigned to multiple classes" in e for e in validate_fixture(mutant)))

    def test_expected_option_sets_are_recomputed(self):
        mutant = copy.deepcopy(self.fixture)
        mutant["expected_findings"][0]["expected_robust_option_set"] = ["brake"]
        self.assertTrue(any("robust option-set mismatch" in e for e in validate_fixture(mutant)))

    def test_timed_query_must_bind_a_successor_receipt(self):
        mutant = copy.deepcopy(self.fixture)
        mutant["measurement"]["outcomes"][0]["delivered_observation_receipt_id"] = "forged-receipt"
        self.assertTrue(any("matching successor cell" in e for e in validate_fixture(mutant)))

    def test_late_query_cannot_create_an_observation_class(self):
        mutant = copy.deepcopy(self.fixture)
        late = next(x for x in mutant["measurement"]["outcomes"] if x["delivery_status"] == "late")
        late["delivered_observation_class"] = "late-branch"
        late["delivered_observation_receipt_id"] = "late-receipt"
        self.assertTrue(any("non-timely" in e for e in validate_fixture(mutant)))

    def test_incomplete_query_status_census_is_rejected(self):
        mutant = copy.deepcopy(self.fixture)
        mutant["measurement"]["outcomes"].pop()
        self.assertTrue(any("exactly one outcome" in e for e in validate_fixture(mutant)))

    def test_missing_negative_control_is_rejected(self):
        mutant = copy.deepcopy(self.fixture)
        mutant["negative_controls"] = [
            x for x in mutant["negative_controls"] if x["control_id"] != "future-command-mutation"
        ]
        self.assertTrue(any("missing required negative controls" in e for e in validate_fixture(mutant)))


if __name__ == "__main__":
    unittest.main()
