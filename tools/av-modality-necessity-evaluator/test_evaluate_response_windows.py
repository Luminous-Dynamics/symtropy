import copy
import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = Path(__file__).with_name("evaluate_response_windows.py")
spec = importlib.util.spec_from_file_location("response_window_evaluator", MODULE_PATH)
evaluator = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(evaluator)


def load_fixture(relative_path):
    with (ROOT / relative_path).open("r", encoding="utf-8") as handle:
        return json.load(handle)


class ResponseWindowEvaluatorTests(unittest.TestCase):
    def setUp(self):
        self.workload = load_fixture(
            "research/av-response-window-silence-v0/fixtures/agent/workload-v1.json"
        )
        self.truth = load_fixture(
            "research/av-response-window-silence-v0/fixtures/evaluator-only/truth-v1.json"
        )
        self.submission = load_fixture(
            "research/av-response-window-silence-v0/fixtures/agent/reference-submission-v1.json"
        )

    def test_reference_policy_passes_all_synthetic_decision_windows(self):
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        totals = score["totals"]
        self.assertEqual(totals["episode_count"], 12)
        self.assertEqual(totals["decision_count"], 15)
        self.assertEqual(totals["correct_decision_count"], 15)
        self.assertEqual(totals["decision_accuracy"], 1.0)
        self.assertEqual(totals["required_response_count"], 4)
        self.assertEqual(totals["response_window_hits"], 4)
        self.assertEqual(totals["response_window_rate"], 1.0)
        self.assertEqual(totals["silence_false_positives"], 0)
        self.assertEqual(totals["silence_precision"], 1.0)
        self.assertEqual(totals["unsolicited_responses"], 0)
        self.assertEqual(totals["duplicate_responses"], 0)
        self.assertEqual(totals["yield_ack_failures"], 0)
        self.assertEqual(totals["stop_ack_failures"], 0)
        self.assertEqual(totals["stop_latch_violations"], 0)
        self.assertEqual(totals["backchannel_false_yields"], 0)
        self.assertEqual(totals["stale_responses"], 0)
        self.assertEqual(totals["background_window_misses"], 0)
        self.assertTrue(all(not row["violations"] for row in score["per_episode"]))

    def test_unsolicited_speech_in_dead_time_is_false_positive(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rw-001-dead-time")
        row["actions"].append({
            "action_id": "spam-during-dead-time",
            "trigger_event_id": "noise-1",
            "action_type": "respond",
            "decision_tick": 11,
            "output_start_tick": 12,
            "output_end_tick": 14,
            "turn_epoch": "epoch-rw001",
            "evidence_refs": ["obs-noise-1"],
            "confidence": 0.9,
        })
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["silence_false_positives"], 1)
        self.assertEqual(score["totals"]["unsolicited_responses"], 1)
        self.assertLess(score["totals"]["silence_precision"], 1.0)

    def test_response_at_window_end_is_late_not_on_time(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rw-002-urgent-hazard")
        row["actions"][0]["output_start_tick"] = 19
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["late_responses"], 1)
        self.assertEqual(score["totals"]["response_window_hits"], 3)
        self.assertEqual(score["totals"]["missed_responses"], 0)

    def test_response_before_window_is_early(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rw-002-urgent-hazard")
        row["actions"][0]["output_start_tick"] = 11
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["early_responses"], 1)
        self.assertEqual(score["totals"]["response_window_hits"], 3)

    def test_duplicate_reply_to_same_trigger_is_counted(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rw-003-duplicate-trigger")
        duplicate = copy.deepcopy(row["actions"][0])
        duplicate.update(action_id="second-alarm-response",decision_tick=10,output_start_tick=11,output_end_tick=12)
        row["actions"].append(duplicate)
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["duplicate_responses"], 1)

    def test_backchannel_false_yield_is_not_silence(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rw-006-backchannel")
        row["actions"].append({
            "action_id": "incorrect-yield",
            "trigger_event_id": "user-backchannel",
            "action_type": "yield",
            "decision_tick": 19,
            "output_stop_ack_tick": 20,
            "turn_epoch": "epoch-rw006",
            "evidence_refs": ["obs-backchannel-6"],
        })
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["backchannel_false_yields"], 1)
        result = next(x for x in score["per_episode"] if x["episode_id"] == "rw-006-backchannel")
        self.assertLess(result["correct_decision_count"], result["decision_count"])

    def test_speech_after_latching_stop_is_a_violation(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rw-007-latching-stop")
        row["actions"].append({
            "action_id": "unauthorized-resume",
            "trigger_event_id": "tempting-followup",
            "action_type": "respond",
            "decision_tick": 36,
            "output_start_tick": 37,
            "turn_epoch": "epoch-rw007",
            "evidence_refs": ["obs-followup-7"],
        })
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["stop_latch_violations"], 1)

    def test_response_after_supersession_is_stale(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rw-008-superseded-response")
        row["actions"].append({
            "action_id": "stale-old-plan-output",
            "trigger_event_id": "old-trigger",
            "action_type": "respond",
            "decision_tick": 24,
            "output_start_tick": 25,
            "turn_epoch": "epoch-rw008",
            "evidence_refs": ["obs-old-trigger-8"],
        })
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["stale_responses"], 1)

    def test_yield_acknowledgement_late_is_a_failure(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rw-005-user-interruption")
        row["actions"][0]["output_stop_ack_tick"] = 25
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["yield_ack_failures"], 1)

    def test_background_load_deadline_miss_is_reported(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rw-010-background-load")
        row["actions"][0]["output_start_tick"] = 41
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["background_load_episodes"], 2)
        self.assertEqual(score["totals"]["background_window_misses"], 1)

    def test_supersession_cancel_must_target_prior_plan(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rw-008-superseded-response")
        row["actions"][0]["target_action_id"] = "wrong-plan"
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        result = next(x for x in score["per_episode"] if x["episode_id"] == "rw-008-superseded-response")
        self.assertLess(result["correct_decision_count"], result["decision_count"])

    def test_output_before_trigger_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rw-002-urgent-hazard")
        row["actions"][0]["decision_tick"] = 9
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        result = next(x for x in score["per_episode"] if x["episode_id"] == "rw-002-urgent-hazard")
        self.assertTrue(any("decision_precedes_trigger" in x for x in result["violations"]))
        self.assertLess(result["correct_decision_count"], result["decision_count"])

    def test_candidate_must_cover_exact_episode_set(self):
        candidate = copy.deepcopy(self.submission)
        candidate["predictions"].pop()
        with self.assertRaises(evaluator.EvaluationError):
            evaluator.evaluate(self.workload, self.truth, candidate)


if __name__ == "__main__":
    unittest.main()