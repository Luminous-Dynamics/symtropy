import copy
import json
import unittest
from pathlib import Path

import action_space_evsi as evaluator

BASE = Path(__file__).parent / "fixtures"


def load(path):
    return json.loads((BASE / path).read_text(encoding="utf-8"))


class ActionSpaceEVSITests(unittest.TestCase):
    def setUp(self):
        self.workload = load("agent/workload-v1.json")
        self.truth = load("evaluator-only/potential-outcomes-v1.json")
        self.submission = load("agent/reference-submission-v1.json")

    def score(self, submission=None):
        return evaluator.evaluate(self.workload, self.truth, submission or self.submission)

    def test_reference_scores_every_action_and_stop(self):
        score = self.score()
        self.assertEqual(score["totals"]["action_space_coverage"], 1.0)
        self.assertEqual(score["totals"]["forecast_count"], 9)
        self.assertEqual(score["totals"]["missed_best_action_count"], 0)
        self.assertEqual(score["totals"]["decision_regret_total"], 0.0)

    def test_missing_unselected_action_forecast_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        candidate["predictions"][0]["forecasts"] = [
            row for row in candidate["predictions"][0]["forecasts"]
            if row["action_id"] != "zoom-detail"
        ]
        with self.assertRaises(evaluator.EvaluationError):
            self.score(candidate)

    def test_duplicate_action_forecast_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        candidate["predictions"][0]["forecasts"].append(
            copy.deepcopy(candidate["predictions"][0]["forecasts"][0])
        )
        with self.assertRaises(evaluator.EvaluationError):
            self.score(candidate)

    def test_unknown_selected_action_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        candidate["predictions"][0]["selected_action_id"] = "hidden-action"
        with self.assertRaises(evaluator.EvaluationError):
            self.score(candidate)

    def test_stale_transcript_head_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        candidate["predictions"][1]["transcript_head_sha256"] = "f" * 64
        with self.assertRaises(evaluator.EvaluationError):
            self.score(candidate)

    def test_forecast_after_current_tick_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        candidate["predictions"][1]["forecast_tick"] += 1
        with self.assertRaises(evaluator.EvaluationError):
            self.score(candidate)

    def test_deadline_infeasible_action_cannot_be_selected(self):
        candidate = copy.deepcopy(self.submission)
        candidate["predictions"][2]["selected_action_id"] = "independent-visual-check"
        with self.assertRaises(evaluator.EvaluationError):
            self.score(candidate)

    def test_unselected_forecast_contributes_to_calibration(self):
        before = self.score()["totals"]["gross_evsi_mae"]
        candidate = copy.deepcopy(self.submission)
        row = next(
            x for x in candidate["predictions"][1]["forecasts"]
            if x["action_id"] == "zoom-busy-dashboard"
        )
        row.update(estimated_gross_evsi=1.8, lower_gross_evsi=1.7, upper_gross_evsi=1.9)
        after_score = self.score(candidate)
        self.assertEqual(after_score["episodes"][1]["selected_action_id"], "listen-short-alarm-cue")
        self.assertGreater(after_score["totals"]["gross_evsi_mae"], before)

    def test_high_information_zero_value_correctly_stops(self):
        score = self.score()
        self.assertEqual(score["episodes"][0]["oracle_best_action_ids"], ["__stop__"])
        self.assertEqual(score["episodes"][0]["selected_action_id"], "__stop__")

    def test_low_information_high_value_cue_is_selected(self):
        score = self.score()
        self.assertEqual(score["episodes"][1]["oracle_best_action_ids"], ["listen-short-alarm-cue"])
        self.assertEqual(score["episodes"][1]["selected_action_id"], "listen-short-alarm-cue")

    def test_correlated_low_value_and_late_high_value_do_not_override_stop(self):
        score = self.score()
        self.assertEqual(score["episodes"][2]["oracle_best_action_ids"], ["__stop__"])
        self.assertEqual(score["episodes"][2]["selected_action_id"], "__stop__")


if __name__ == "__main__":
    unittest.main()
