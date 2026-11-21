import copy
import importlib.util
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("evaluate.py")
spec = importlib.util.spec_from_file_location("active_sensing_evaluator", MODULE_PATH)
evaluator = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(evaluator)

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "research" / "av-active-sensing-decision-value-v0" / "fixtures"


class ActiveSensingEvaluatorTests(unittest.TestCase):
    def setUp(self):
        self.workload = evaluator.read_json(BASE / "agent" / "workload-v1.json")
        self.truth = evaluator.read_json(BASE / "evaluator-only" / "action-truth-v1.json")
        self.submission = evaluator.read_json(BASE / "agent" / "reference-submission-v1.json")

    def score(self, submission=None, workload=None, truth=None):
        return evaluator.evaluate(workload or self.workload, truth or self.truth, submission or self.submission)

    def prediction(self, submission, episode_id):
        return next(row for row in submission["predictions"] if row["episode_id"] == episode_id)

    def result(self, score, episode_id):
        return next(row for row in score["per_episode"] if row["episode_id"] == episode_id)

    def test_reference_policy_matches_all_optimal_actions(self):
        score = self.score()
        self.assertEqual(score["totals"]["episode_count"], 8)
        self.assertEqual(score["totals"]["answer_accuracy"], 1.0)
        self.assertEqual(score["totals"]["safe_decision_accuracy"], 1.0)
        self.assertEqual(score["totals"]["decision_regret_total"], 0)
        self.assertEqual(score["totals"]["acquisition_count"], 3)
        self.assertEqual(score["totals"]["wasted_acquisition_count"], 0)
        self.assertEqual(score["totals"]["late_acquisition_count"], 0)
        self.assertEqual(score["totals"]["budget_violation_count"], 0)
        self.assertEqual(score["totals"]["evidence_violation_count"], 0)
        self.assertEqual(score["totals"]["mean_realized_evsi"], 15 / 8)

    def test_high_eig_zero_evsi_acquisition_is_wasted(self):
        candidate = copy.deepcopy(self.submission)
        row = self.prediction(candidate, "as-001-high-eig-zero-evsi")
        row["action_requests"] = [{
            "request_id": "waste-high-eig", "action_id": "inspect-uniform-label",
            "request_tick": 10, "estimated_eig_nats": 1.4,
            "estimated_evsi": 0, "estimated_cost_joules": 0.8
        }]
        score = self.score(candidate)
        result = self.result(score, "as-001-high-eig-zero-evsi")
        self.assertEqual(result["wasted_acquisition_count"], 1)
        self.assertEqual(result["decision_regret"], 0.8)

    def test_low_eig_high_evsi_observation_is_missed_if_not_requested(self):
        candidate = copy.deepcopy(self.submission)
        self.prediction(candidate, "as-002-low-eig-high-evsi")["action_requests"] = []
        score = self.score(candidate)
        result = self.result(score, "as-002-low-eig-high-evsi")
        self.assertEqual(result["decision_regret"], 6)
        self.assertFalse(result["action_policy_correct"])

    def test_late_observation_cannot_be_cited(self):
        candidate = copy.deepcopy(self.submission)
        row = self.prediction(candidate, "as-005-deadline-late-observation")
        row["action_requests"] = [{
            "request_id": "late-camera", "action_id": "request-delayed-camera-frame",
            "request_tick": 10, "estimated_eig_nats": 1.2,
            "estimated_evsi": 1, "estimated_cost_joules": 0.6
        }]
        row["response_tick"] = 26
        row["evidence_as_of_tick"] = 25
        row["cited_observation_refs"] = ["obs-delayed-frame"]
        score = self.score(candidate)
        result = self.result(score, "as-005-deadline-late-observation")
        self.assertIn("post_deadline_observation_cited", result["evidence_violations"])
        self.assertEqual(result["late_acquisition_count"], 1)

    def test_fabricated_observation_reference_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        self.prediction(candidate, "as-001-high-eig-zero-evsi")["cited_observation_refs"].append("made-up")
        result = self.result(self.score(candidate), "as-001-high-eig-zero-evsi")
        self.assertIn("fabricated_or_unrequested_observation", result["evidence_violations"])

    def test_common_cause_claim_is_not_independent(self):
        candidate = copy.deepcopy(self.submission)
        row = self.prediction(candidate, "as-004-common-cause-duplicate")
        row["action_requests"] = [{
            "request_id": "same-event-audio", "action_id": "capture-impact-audio",
            "request_tick": 20, "estimated_eig_nats": 0.3,
            "estimated_evsi": 0, "estimated_cost_joules": 0.2
        }]
        row["response_tick"] = 24
        row["evidence_as_of_tick"] = 23
        row["cited_observation_refs"] = ["obs-impact-video", "obs-impact-audio"]
        row["claimed_independent_corroboration"] = True
        result = self.result(self.score(candidate), "as-004-common-cause-duplicate")
        self.assertIn("common_cause_evidence_claimed_independent", result["evidence_violations"])

    def test_budget_overrun_fails_closed(self):
        candidate = copy.deepcopy(self.submission)
        row = self.prediction(candidate, "as-002-low-eig-high-evsi")
        row["action_requests"] = [
            {"request_id": f"req-{i}", "action_id": "inspect-hazard-tone",
             "request_tick": 10, "estimated_eig_nats": 0.1,
             "estimated_evsi": 1, "estimated_cost_joules": 0.4}
            for i in range(2)
        ]
        result = self.result(self.score(candidate), "as-002-low-eig-high-evsi")
        self.assertIn("max_actions_exceeded", result["budget_violations"])
        self.assertIn("audio_byte_budget_exceeded", result["budget_violations"])

    def test_partial_sensor_action_mints_no_observation(self):
        candidate = copy.deepcopy(self.submission)
        row = self.prediction(candidate, "as-007-partial-unavailable-action")
        row["action_requests"] = [{
            "request_id": "partial-pump", "action_id": "request-pump-audio",
            "request_tick": 10, "estimated_eig_nats": 0.1,
            "estimated_evsi": 0, "estimated_cost_joules": 0.4
        }]
        row["response_tick"] = 16
        row["evidence_as_of_tick"] = 15
        row["cited_observation_refs"].append("obs-pump-audio")
        result = self.result(self.score(candidate), "as-007-partial-unavailable-action")
        self.assertIn("fabricated_or_unrequested_observation", result["evidence_violations"])

    def test_duplicate_action_requests_are_counted_and_capped(self):
        candidate = copy.deepcopy(self.submission)
        row = self.prediction(candidate, "as-002-low-eig-high-evsi")
        row["action_requests"] = [
            {"request_id": f"repeat-{i}", "action_id": "inspect-hazard-tone",
             "request_tick": 10, "estimated_eig_nats": 0.1,
             "estimated_evsi": 1, "estimated_cost_joules": 0.4}
            for i in range(2)
        ]
        result = self.result(self.score(candidate), "as-002-low-eig-high-evsi")
        self.assertIn("action_max_uses_exceeded", result["evidence_violations"])
