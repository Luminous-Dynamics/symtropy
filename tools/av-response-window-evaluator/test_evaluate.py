import copy
import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "research" / "av-response-window-v0"

MODULE_PATH = Path(__file__).with_name("evaluate.py")
spec = importlib.util.spec_from_file_location("response_window_evaluator", MODULE_PATH)
evaluator = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(evaluator)


class ResponseWindowEvaluatorTests(unittest.TestCase):
    def setUp(self):
        self.workload = evaluator.read_json(BASE / "fixtures" / "agent" / "workload-v1.json")
        self.truth = evaluator.read_json(BASE / "fixtures" / "evaluator-only" / "truth-v1.json")
        self.submission = evaluator.read_json(BASE / "fixtures" / "agent" / "reference-submission-v1.json")

    def score(self, workload=None, truth=None, submission=None):
        return evaluator.evaluate(
            workload if workload is not None else self.workload,
            truth if truth is not None else self.truth,
            submission if submission is not None else self.submission,
        )

    def synthetic_response(self, response_id, episode_id, trigger_ref, start, end, content,
                            end_kind="playback_ended", stop_request_ref=None):
        response = {
            "response_id": response_id,
            "episode_id": episode_id,
            "assigned_trigger_ref": trigger_ref,
            "response_start_tick": start,
            "response_end_tick": end,
            "content": content,
            "playback_start_event_ref": f"render-start-{response_id}",
            "playback_end_event_ref": f"render-end-{response_id}",
            "playback_end_kind": end_kind,
            "controller_permission_event_ref": f"controller-permit-{response_id}",
        }
        if stop_request_ref is not None:
            response["output_stop_request_event_ref"] = stop_request_ref
        return response

    def episode(self, report, episode_id):
        return next(row for row in report["per_episode"] if row["episode_id"] == episode_id)

    def test_reference_known_answer_scores_all_expected_events(self):
        report = self.score()
        totals = report["totals"]
        self.assertEqual(totals["opportunity_count"], 11)
        self.assertEqual(totals["response_required_count"], 8)
        self.assertEqual(totals["expected_silence_count"], 3)
        self.assertEqual(totals["interaction_decision_correct_count"], 11)
        self.assertEqual(totals["on_time_count"], 8)
        self.assertEqual(totals["early_count"], 0)
        self.assertEqual(totals["late_count"], 0)
        self.assertEqual(totals["missed_count"], 0)
        self.assertEqual(totals["false_intervention_count"], 0)
        self.assertEqual(totals["content_correct_count"], 8)
        self.assertEqual(totals["window_response_rate"], 1.0)
        self.assertEqual(totals["content_accuracy"], 1.0)
        self.assertEqual(totals["hard_violation_count"], 0)

    def test_exact_window_start_is_on_time_and_end_is_late(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["responses"] if x["response_id"] == "r1")
        row["response_start_tick"] = 10
        row["response_end_tick"] = 12
        report = self.score(submission=candidate)
        result = self.episode(report, "rw-001-spi-on-time")
        self.assertEqual(result["on_time_count"], 1)
        self.assertEqual(result["early_count"], 0)
        self.assertEqual(result["late_count"], 0)

        row["response_start_tick"] = 20
        row["response_end_tick"] = 22
        report = self.score(submission=candidate)
        result = self.episode(report, "rw-001-spi-on-time")
        self.assertEqual(result["late_count"], 1)
        self.assertEqual(result["on_time_count"], 0)
        self.assertEqual(report["totals"]["window_response_rate"], 7 / 8)

    def test_response_before_window_is_early_not_on_time(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["responses"] if x["response_id"] == "r1")
        row["response_start_tick"] = 9
        row["response_end_tick"] = 11
        report = self.score(submission=candidate)
        result = self.episode(report, "rw-001-spi-on-time")
        self.assertEqual(result["early_count"], 1)
        self.assertEqual(result["on_time_count"], 0)
        self.assertEqual(report["totals"]["window_response_rate"], 7 / 8)

    def test_expected_silence_response_is_false_intervention(self):
        candidate = copy.deepcopy(self.submission)
        candidate["responses"].append(self.synthetic_response(
            "false-silence", "rw-007-expected-silence", "silence-7", 12, 14, "unsolicited"
        ))
        report = self.score(submission=candidate)
        result = self.episode(report, "rw-007-expected-silence")
        self.assertEqual(result["false_intervention_count"], 1)
        self.assertEqual(report["totals"]["false_intervention_count"], 1)
        self.assertLess(report["totals"]["expected_silence_specificity"], 1.0)

    def test_response_after_cancellation_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        candidate["responses"].append(self.synthetic_response(
            "stale-after-cancel", "rw-008-canceled-trigger", "trig-8", 16, 18,
            "stale intervention", end_kind="output_stopped",
            stop_request_ref="stop-request-stale-after-cancel"
        ))
        report = self.score(submission=candidate)
        result = self.episode(report, "rw-008-canceled-trigger")
        self.assertIn("response_starts_after_cancellation", result["violations"])
        self.assertEqual(result["false_intervention_count"], 1)

    def test_cancellation_cannot_precede_a_primary_trigger(self):
        workload = copy.deepcopy(self.workload)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "rw-008-canceled-trigger")
        episode["stream_events"] = [
            event for event in episode["stream_events"] if event["kind"] != "trigger"
        ]
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)

    def test_response_that_overhangs_cancellation_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        candidate["responses"].append(self.synthetic_response(
            "crosses-cancel", "rw-008-canceled-trigger", "trig-8", 14, 16,
            "canceled while speaking", end_kind="output_stopped",
            stop_request_ref="stop-request-crosses-cancel"
        ))
        report = self.score(submission=candidate)
        result = self.episode(report, "rw-008-canceled-trigger")
        self.assertIn("response_continues_after_cancellation", result["violations"])

    def test_two_responses_for_one_opportunity_are_duplicate_interventions(self):
        candidate = copy.deepcopy(self.submission)
        candidate["responses"].append(self.synthetic_response(
            "duplicate-r1", "rw-001-spi-on-time", "trig-1", 14, 16, "safety_correction"
        ))
        report = self.score(submission=candidate)
        result = self.episode(report, "rw-001-spi-on-time")
        self.assertIn("multiple_responses_assigned_to_one_opportunity", result["violations"])
        self.assertEqual(result["duplicate_response_count"], 1)

    def test_single_output_channel_cannot_overlap_its_own_responses(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["responses"] if x["response_id"] == "r4a")
        row["response_end_tick"] = 21
        report = self.score(submission=candidate)
        result = self.episode(report, "rw-004-overlapping-windows")
        self.assertIn("assistant_response_intervals_overlap", result["violations"])

    def test_window_cannot_precede_trigger_or_query(self):
        truth = copy.deepcopy(self.truth)
        opp = truth["episodes"][0]["opportunities"][0]
        opp["window_start_tick"] = 4
        with self.assertRaises(evaluator.EvaluationError):
            self.score(truth=truth)

    def test_every_truth_trigger_must_be_visible_to_candidate(self):
        workload = copy.deepcopy(self.workload)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "rw-007-expected-silence")
        episode["stream_events"][0].pop("trigger_ref")
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)

    def test_duplicate_response_ids_are_rejected(self):
        candidate = copy.deepcopy(self.submission)
        duplicate = copy.deepcopy(candidate["responses"][0])
        duplicate["episode_id"] = "rw-002-vti-start-boundary"
        candidate["responses"].append(duplicate)
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_unknown_trigger_assignment_is_a_hard_violation(self):
        candidate = copy.deepcopy(self.submission)
        candidate["responses"][0]["assigned_trigger_ref"] = "not-a-trigger"
        report = self.score(submission=candidate)
        result = self.episode(report, "rw-001-spi-on-time")
        self.assertIn("response_assigned_to_unknown_trigger", result["violations"])


    def test_response_audio_overhang_past_episode_deadline_is_counted(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["responses"] if x["response_id"] == "r1")
        row.update(response_start_tick=28, response_end_tick=31)
        report = self.score(submission=candidate)
        result = self.episode(report, "rw-001-spi-on-time")
        self.assertIn("response_overhang_past_episode_deadline", result["violations"])
        self.assertEqual(result["deadline_miss_count"], 1)
        self.assertEqual(report["totals"]["deadline_miss_count"], 1)

    def test_duplicate_delivery_cannot_exist_without_primary_trigger(self):
        workload = copy.deepcopy(self.workload)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "rw-009-duplicate-trigger")
        episode["stream_events"] = [
            event for event in episode["stream_events"] if event["kind"] != "trigger"
        ]
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)

    def test_response_start_cannot_precede_trigger_availability(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["responses"] if x["response_id"] == "r1")
        row.update(response_start_tick=4, response_end_tick=6)
        report = self.score(submission=candidate)
        result = self.episode(report, "rw-001-spi-on-time")
        self.assertIn("response_precedes_trigger_availability", result["violations"])
        self.assertEqual(result["early_count"], 1)

    def test_controller_permission_without_playback_start_does_not_create_response(self):
        candidate = copy.deepcopy(self.submission)
        candidate["responses"][0].pop("playback_start_event_ref")
        candidate["responses"][0]["controller_permission_event_ref"] = "permission-only"
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_renderer_output_stop_requires_linked_request_receipt(self):
        candidate = copy.deepcopy(self.submission)
        response = candidate["responses"][0]
        response["playback_end_kind"] = "output_stopped"
        response.pop("output_stop_request_event_ref", None)
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_renderer_lifecycle_event_references_must_be_unique(self):
        candidate = copy.deepcopy(self.submission)
        candidate["responses"][1]["playback_start_event_ref"] = candidate["responses"][0]["playback_start_event_ref"]
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_renderer_qualified_mode_requires_digest_clock_relation(self):
        candidate = copy.deepcopy(self.submission)
        candidate["timing_evidence_mode"] = "renderer_qualified"
        candidate["clock_relation_ref"] = "unqualified-clock-map"
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_output_stop_request_ref_cannot_be_reused(self):
        candidate = copy.deepcopy(self.submission)
        first = candidate["responses"][0]
        first["playback_end_kind"] = "output_stopped"
        first["output_stop_request_event_ref"] = "stop-req-shared"
        second = candidate["responses"][1]
        second["playback_end_kind"] = "output_stopped"
        second["output_stop_request_event_ref"] = "stop-req-shared"
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_output_end_event_ref_must_not_equal_start_event_ref(self):
        candidate = copy.deepcopy(self.submission)
        candidate["responses"][0]["playback_end_event_ref"] = candidate["responses"][0]["playback_start_event_ref"]
        with self.assertRaises(evaluator.EvaluationError):
            self.score(submission=candidate)

    def test_response_interval_must_have_nonnegative_duration(self):
        candidate = copy.deepcopy(self.submission)
        candidate["responses"][0]["response_end_tick"] = candidate["responses"][0]["response_start_tick"] - 1
        report = self.score(submission=candidate)
        result = self.episode(report, "rw-001-spi-on-time")
        self.assertIn("invalid_response_interval", result["violations"])


if __name__ == "__main__":
    unittest.main()
