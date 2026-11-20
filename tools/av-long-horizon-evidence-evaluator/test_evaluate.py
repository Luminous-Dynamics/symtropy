import copy
import importlib.util
import json
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("evaluate.py")
spec = importlib.util.spec_from_file_location("long_horizon_evaluator", MODULE_PATH)
evaluator = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(evaluator)

ROOT = Path(__file__).resolve().parents[2]
WORKLOAD = json.loads((ROOT / "research/av-long-horizon-evidence-v0/fixtures/agent/workload-v1.json").read_text(encoding="utf-8"))
TRUTH = json.loads((ROOT / "research/av-long-horizon-evidence-v0/fixtures/evaluator-only/truth-v1.json").read_text(encoding="utf-8"))
SUBMISSION = json.loads((ROOT / "research/av-long-horizon-evidence-v0/fixtures/agent/reference-submission-v1.json").read_text(encoding="utf-8"))


class LongHorizonEvaluatorTests(unittest.TestCase):
    def setUp(self):
        self.workload = copy.deepcopy(WORKLOAD)
        self.truth = copy.deepcopy(TRUTH)
        self.submission = copy.deepcopy(SUBMISSION)

    def prediction(self, episode_id):
        return next(row for row in self.submission["predictions"] if row["episode_id"] == episode_id)

    def result(self, score, episode_id):
        return next(row for row in score["per_episode"] if row["episode_id"] == episode_id)

    def test_reference_submission_matches_finite_known_answers(self):
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        self.assertEqual(score["totals"]["episode_count"], 9)
        self.assertEqual(score["totals"]["answer_correct_count"], 8)
        self.assertEqual(score["totals"]["decision_correct_count"], 9)
        self.assertEqual(score["totals"]["citations_complete_count"], 9)
        self.assertEqual(score["totals"]["hard_violation_count"], 0)
        self.assertEqual(score["totals"]["deadline_miss_count"], 0)

    def test_duplicate_delivery_reuses_one_source_observation(self):
        episode = next(x for x in self.workload["episodes"] if x["episode_id"] == "lh-002-repeated-event-dedup")
        observations, event_ids, first_available, _, _ = evaluator._event_index(episode)
        self.assertEqual(len(observations), 4)
        self.assertEqual(len(event_ids["obs-tap-2"]), 2)
        self.assertEqual(first_available["obs-tap-2"], 90)
        self.assertEqual(self.prediction("lh-002-repeated-event-dedup")["claimed_distinct_event_count"], 4)

    def test_duplicate_delivery_with_changed_source_identity_is_rejected(self):
        episode = next(x for x in self.workload["episodes"] if x["episode_id"] == "lh-002-repeated-event-dedup")
        replay = next(x for x in episode["stream_events"] if x["event_id"] == "ev-tap-2-replay")
        replay["artifact_sha256"] = "f" * 64
        with self.assertRaises(evaluator.EvaluationError):
            evaluator._event_index(episode)

    def test_correction_keeps_historical_source_but_only_current_revision_is_current(self):
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        row = self.result(score, "lh-003-correction-supersession")
        self.assertTrue(row["answer_correct"])
        self.assertTrue(row["citations_complete"])
        self.assertEqual(row["violations"], [])

    def test_superseded_observation_cannot_be_claimed_current(self):
        prediction = self.prediction("lh-003-correction-supersession")
        prediction["current_observation_refs"] = ["obs-machine-state-v1"]
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        row = self.result(score, "lh-003-correction-supersession")
        self.assertIn("superseded_observation_claimed_current", row["violations"])

    def test_revoked_observation_cannot_be_cited_after_revocation(self):
        prediction = self.prediction("lh-008-memory-revocation")
        prediction["cited_observation_refs"].append("obs-private-utterance")
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        row = self.result(score, "lh-008-memory-revocation")
        self.assertIn("revoked_observation_cited", row["violations"])

    def test_common_cause_views_cannot_be_claimed_independent(self):
        self.prediction("lh-004-common-cause-dependence")["claimed_independent_corroboration"] = True
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        row = self.result(score, "lh-004-common-cause-dependence")
        self.assertIn("common_cause_views_claimed_as_independent", row["violations"])

    def test_incomparable_clocks_cannot_be_claimed_ordered(self):
        self.prediction("lh-007-cross-clock-and-event-order")["claimed_cross_clock_order"] = True
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        row = self.result(score, "lh-007-cross-clock-and-event-order")
        self.assertIn("incomparable_clocks_claimed_as_ordered", row["violations"])

    def test_absent_event_requires_abstention(self):
        prediction = self.prediction("lh-006-absent-event-false-memory")
        prediction.update(decision="answer", answer="glass-breaking", confidence=0.9)
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        row = self.result(score, "lh-006-absent-event-false-memory")
        self.assertFalse(row["decision_correct"])
        self.assertFalse(row["answer_correct"])

    def test_absent_event_false_memory_cannot_cite_unrelated_source_as_support(self):
        prediction = self.prediction("lh-006-absent-event-false-memory")
        prediction["cited_observation_refs"] = ["obs-door-close"]
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        row = self.result(score, "lh-006-absent-event-false-memory")
        self.assertFalse(row["decision_correct"])
        self.assertIn("absent_event_abstention_must_not_invent_source", row["violations"])

    def test_required_source_citation_cannot_be_omitted(self):
        prediction = self.prediction("lh-001-delayed-recall-10m")
        prediction["cited_observation_refs"] = []
        prediction["current_observation_refs"] = []
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        row = self.result(score, "lh-001-delayed-recall-10m")
        self.assertFalse(row["citations_complete"])
        self.assertIn("required_source_evidence_missing", row["violations"])

    def test_query_cutoff_cannot_be_moved_back_to_hide_available_evidence(self):
        self.prediction("lh-001-delayed-recall-10m")["evidence_as_of_tick"] = 0
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        row = self.result(score, "lh-001-delayed-recall-10m")
        self.assertIn("evidence_cutoff_must_equal_query_tick", row["violations"])

    def test_response_after_deadline_is_reported(self):
        prediction = self.prediction("lh-001-delayed-recall-10m")
        prediction["response_tick"] = 999
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        row = self.result(score, "lh-001-delayed-recall-10m")
        self.assertTrue(row["deadline_miss"])

    def test_duplicate_citations_are_rejected(self):
        prediction = self.prediction("lh-001-delayed-recall-10m")
        prediction["cited_observation_refs"] = ["obs-door-impact", "obs-door-impact"]
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        row = self.result(score, "lh-001-delayed-recall-10m")
        self.assertIn("duplicate_observation_citation", row["violations"])

    def test_submission_must_cover_every_episode_exactly_once(self):
        self.submission["predictions"].pop()
        with self.assertRaises(evaluator.EvaluationError):
            evaluator.evaluate(self.workload, self.truth, self.submission)


if __name__ == "__main__":
    unittest.main()