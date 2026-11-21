import copy
import importlib.util
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("evaluate.py")
spec = importlib.util.spec_from_file_location("sequential_eval", MODULE_PATH)
evaluator = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(evaluator)

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "research" / "av-sequential-acquisition-v0" / "fixtures"


class SequentialAcquisitionTests(unittest.TestCase):
    def setUp(self):
        self.workload = evaluator.read_json(BASE / "agent" / "workload-v1.json")
        self.truth = evaluator.read_json(BASE / "evaluator-only" / "truth-v1.json")
        self.submission = evaluator.read_json(BASE / "agent" / "reference-submission-v1.json")

    def score(self, workload=None, truth=None, submission=None):
        return evaluator.evaluate(workload or self.workload, truth or self.truth, submission or self.submission)

    def episode(self, episode_id):
        return next(row for row in self.workload["episodes"] if row["episode_id"] == episode_id)

    def prediction(self, submission, episode_id):
        return next(row for row in submission["predictions"] if row["episode_id"] == episode_id)

    def test_reference_plans_match_exact_oracle(self):
        report = self.score()
        self.assertEqual(report["totals"]["episode_count"], 4)
        self.assertEqual(report["totals"]["oracle_action_set_matches"], 4)
        self.assertAlmostEqual(report["totals"]["mean_regret_vs_exact_nonadaptive_oracle"], 0.0, places=10)

    def test_eig_trap_has_higher_eig_but_lower_decision_value(self):
        episode = self.episode("sa-001-eig-trap")
        subtype = evaluator.plan_statistics(episode, ["subtype_detail"])
        hazard = evaluator.plan_statistics(episode, ["hazard_cue"])
        self.assertGreater(subtype["expected_information_gain_bits"], hazard["expected_information_gain_bits"])
        self.assertAlmostEqual(subtype["gross_evsi_units"], 0.0, places=10)
        self.assertGreater(hazard["gross_evsi_units"], 0.5)
        self.assertEqual(evaluator.exact_oracle(episode)["action_ids"], ["hazard_cue"])

    def test_complementary_measurements_have_zero_singleton_evsi(self):
        episode = self.episode("sa-002-sequential-complementarity")
        x = evaluator.plan_statistics(episode, ["read_x"])
        y = evaluator.plan_statistics(episode, ["read_y"])
        xy = evaluator.plan_statistics(episode, ["read_x", "read_y"])
        self.assertAlmostEqual(x["gross_evsi_units"], 0.0, places=10)
        self.assertAlmostEqual(y["gross_evsi_units"], 0.0, places=10)
        self.assertGreater(xy["net_evsi_units"], 0.2)
        self.assertEqual(evaluator.exact_oracle(episode)["action_ids"], ["read_x", "read_y"])

    def test_deadline_inversion_excludes_high_fidelity_sensor(self):
        oracle = evaluator.exact_oracle(self.episode("sa-003-deadline-inversion"))
        self.assertEqual(oracle["action_ids"], ["quick_audio_cue"])
        self.assertLessEqual(oracle["duration_ticks"], 3)

    def test_permission_fallback_excludes_denied_camera(self):
        oracle = evaluator.exact_oracle(self.episode("sa-004-permission-fallback"))
        self.assertEqual(oracle["action_ids"], ["audio_triage"])

    def test_candidate_cannot_request_denied_permission(self):
        candidate = copy.deepcopy(self.submission)
        row = self.prediction(candidate, "sa-004-permission-fallback")
        row["actions"] = [{"action_id":"secure_camera_review","request_tick":0,"arrival_tick":1,"outcome_id":"threat_seen"}]
        row["response_tick"] = 1
        row["decision"] = "contain"
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_candidate_cannot_forge_observation_arrival(self):
        candidate = copy.deepcopy(self.submission)
        row = self.prediction(candidate, "sa-003-deadline-inversion")
        row["actions"][0]["arrival_tick"] = 0
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_candidate_cannot_exceed_resource_budget(self):
        workload = copy.deepcopy(self.workload)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "sa-001-eig-trap")
        episode["budget"]["max_audio_bytes"] = 100
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)

    def test_candidate_outcome_must_match_hidden_realization(self):
        candidate = copy.deepcopy(self.submission)
        self.prediction(candidate, "sa-001-eig-trap")["actions"][0]["outcome_id"] = "no_cue"
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_candidate_decision_must_be_bayes_optimal(self):
        candidate = copy.deepcopy(self.submission)
        self.prediction(candidate, "sa-001-eig-trap")["decision"] = "stay"
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_duplicate_acquisition_action_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        row = self.prediction(candidate, "sa-002-sequential-complementarity")
        row["actions"].append(copy.deepcopy(row["actions"][0]))
        row["actions"][1]["request_tick"] = 1
        row["actions"][1]["arrival_tick"] = 2
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_unhashable_candidate_decision_fails_closed(self):
        candidate = copy.deepcopy(self.submission)
        self.prediction(candidate, "sa-001-eig-trap")["decision"] = []
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_malformed_hidden_state_fails_closed(self):
        truth = copy.deepcopy(self.truth)
        truth["episodes"][0]["hidden_state"] = []
        with self.assertRaises(evaluator.EvaluationError):
            self.score(truth=truth)

    def test_malformed_deadline_fails_closed(self):
        workload = copy.deepcopy(self.workload)
        workload["episodes"][0]["deadline_tick"] = True
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)

    def test_non_object_episode_fails_closed(self):
        workload = copy.deepcopy(self.workload)
        workload["episodes"][0] = []
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)

    def test_outcome_likelihoods_must_normalize(self):
        workload = copy.deepcopy(self.workload)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "sa-001-eig-trap")
        episode["actions"][0]["outcomes"][0]["likelihood_by_state"]["hazard_a"] = 0.2
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)


if __name__ == "__main__":
    unittest.main()