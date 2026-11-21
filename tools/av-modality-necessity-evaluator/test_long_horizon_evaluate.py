import copy
import unittest
from pathlib import Path

import long_horizon_evaluate as evaluator

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "research" / "av-long-horizon-evidence-v0" / "fixtures"


class LongHorizonEvaluatorTests(unittest.TestCase):
    def setUp(self):
        self.workload = evaluator.read_json(BASE / "agent" / "workload-v1.json")
        self.truth = evaluator.read_json(BASE / "evaluator-only" / "truth-v1.json")
        self.submission = evaluator.read_json(BASE / "agent" / "reference-submission-v1.json")

    def score(self, submission=None, workload=None, truth=None):
        return evaluator.evaluate(
            workload or self.workload,
            truth or self.truth,
            submission or self.submission,
        )

    def episode_prediction(self, submission, episode_id):
        return next(x for x in submission["predictions"] if x["episode_id"] == episode_id)

    def episode_result(self, score, episode_id):
        return next(x for x in score["per_episode"] if x["episode_id"] == episode_id)

    def test_reference_submission_passes_all_finite_known_answers(self):
        score = self.score()
        totals = score["totals"]
        self.assertEqual(totals["episode_count"], 12)
        self.assertEqual(totals["answer_correct_count"], 11)
        self.assertEqual(totals["decision_correct_count"], 12)
        self.assertEqual(totals["answer_accuracy"], 11 / 12)
        self.assertEqual(totals["coverage"], 11 / 12)
        self.assertEqual(totals["selective_accuracy"], 1.0)
        self.assertEqual(totals["citation_precision"], 1.0)
        self.assertEqual(totals["citation_recall"], 1.0)
        self.assertEqual(totals["provenance_complete_count"], 12)
        self.assertEqual(totals["evidence_qualified_answer_correct_count"], 11)
        self.assertEqual(totals["evidence_qualified_answer_accuracy"], 11 / 12)
        self.assertEqual(totals["evidence_compliant_count"], 12)
        self.assertEqual(totals["false_memory_count"], 0)
        self.assertEqual(totals["hard_violation_count"], 0)
        self.assertEqual(totals["deadline_miss_count"], 0)

    def test_multisegment_chain_requires_all_fifteen_sources_in_exact_order(self):
        truth = copy.deepcopy(self.truth)
        submission = copy.deepcopy(self.submission)
        episode_id = "lh-012-multisegment-15-source-chain"
        label = next(row for row in truth["episodes"] if row["episode_id"] == episode_id)
        prediction = next(row for row in submission["predictions"] if row["episode_id"] == episode_id)
        self.assertEqual(len(label["required_observation_refs"]), 15)
        self.assertEqual(prediction["current_observation_refs"], label["required_ordered_observation_refs"])
        report = evaluator.evaluate(self.workload, truth, submission)
        result = next(row for row in report["per_episode"] if row["episode_id"] == episode_id)
        self.assertTrue(result["answer_correct"])
        self.assertTrue(result["provenance_complete"])
        prediction["current_observation_refs"] = list(reversed(prediction["current_observation_refs"]))
        prediction["cited_observation_refs"] = list(reversed(prediction["cited_observation_refs"]))
        report = evaluator.evaluate(self.workload, truth, submission)
        result = next(row for row in report["per_episode"] if row["episode_id"] == episode_id)
        self.assertFalse(result["answer_correct"])
        self.assertFalse(result["provenance_complete"])
        self.assertIn("current_observation_order_mismatch", result["violations"])

    def test_correction_is_not_effective_before_its_availability_tick(self):
        score = self.score()
        result = self.episode_result(score, "lh-010-correction-before-arrival")
        self.assertTrue(result["answer_correct"])
        self.assertTrue(result["provenance_complete"])
        self.assertEqual(result["violations"], [])

    def test_revocation_is_not_effective_before_its_availability_tick(self):
        score = self.score()
        result = self.episode_result(score, "lh-011-revocation-before-effect")
        self.assertTrue(result["answer_correct"])
        self.assertTrue(result["provenance_complete"])
        self.assertEqual(result["violations"], [])

    def test_duplicate_transport_reuses_one_source_observation(self):
        score = self.score()
        result = self.episode_result(score, "lh-002-repeated-event-dedup")
        self.assertTrue(result["answer_correct"])
        self.assertTrue(result["provenance_complete"])
        self.assertEqual(result["violations"], [])
        row = self.episode_prediction(self.submission, "lh-002-repeated-event-dedup")
        self.assertEqual(row["cited_observation_refs"].count("obs-tap-2"), 1)

    def test_superseded_observation_cannot_support_current_claim(self):
        candidate = copy.deepcopy(self.submission)
        row = self.episode_prediction(candidate, "lh-003-correction-supersession")
        row["cited_observation_refs"] = ["obs-machine-state-v1"]
        row["current_observation_refs"] = ["obs-machine-state-v1"]
        score = self.score(candidate)
        result = self.episode_result(score, "lh-003-correction-supersession")
        self.assertIn("current_reference_uses_superseded_observation", result["violations"])
        self.assertFalse(result["provenance_complete"])

    def test_revoked_observation_cannot_support_current_claim(self):
        candidate = copy.deepcopy(self.submission)
        row = self.episode_prediction(candidate, "lh-008-memory-revocation")
        row["cited_observation_refs"] = ["obs-private-utterance"]
        row["current_observation_refs"] = ["obs-private-utterance"]
        score = self.score(candidate)
        result = self.episode_result(score, "lh-008-memory-revocation")
        self.assertIn("revoked_evidence_or_derived_memory_reuse", result["violations"])
        self.assertFalse(result["provenance_complete"])

    def test_future_observation_is_not_admissible_at_query_cutoff(self):
        candidate = copy.deepcopy(self.submission)
        episode = next(x for x in self.workload["episodes"] if x["episode_id"] == "lh-001-delayed-recall-10m")
        episode["stream_events"].append({
            "event_id": "ev-future-test", "kind": "observation", "availability_tick": 700,
            "observation_ref": "obs-future-test", "modalities": ["video"], "capture_tick": 700,
            "artifact_sha256": "f" * 64, "clock_domain_id": "test-clock", "clock_generation": 0
        })
        self.episode_prediction(candidate, "lh-001-delayed-recall-10m")["cited_observation_refs"] = ["obs-future-test"]
        result = self.episode_result(self.score(candidate), "lh-001-delayed-recall-10m")
        self.assertIn("citation_not_available_at_evidence_cutoff", result["violations"])

    def test_common_cause_views_cannot_be_claimed_independent(self):
        candidate = copy.deepcopy(self.submission)
        self.episode_prediction(candidate, "lh-004-common-cause-dependence")["claimed_independent_corroboration"] = True
        result = self.episode_result(self.score(candidate), "lh-004-common-cause-dependence")
        self.assertIn("common_cause_views_claimed_independent", result["violations"])

    def test_cross_clock_ordering_requires_qualified_transform(self):
        candidate = copy.deepcopy(self.submission)
        self.episode_prediction(candidate, "lh-007-cross-clock-and-event-order")["claimed_cross_clock_order"] = True
        result = self.episode_result(self.score(candidate), "lh-007-cross-clock-and-event-order")
        self.assertIn("cross_clock_order_claim_without_qualified_transform", result["violations"])

    def test_absent_event_answer_counts_as_false_memory(self):
        candidate = copy.deepcopy(self.submission)
        self.episode_prediction(candidate, "lh-006-absent-event-false-memory").update(
            answer="glass_breaking", decision="answer", confidence=0.9
        )
        result = self.episode_result(self.score(candidate), "lh-006-absent-event-false-memory")
        self.assertFalse(result["decision_correct"])
        self.assertTrue(result["false_memory"])

    def test_missing_required_source_reduces_citation_recall(self):
        candidate = copy.deepcopy(self.submission)
        row = self.episode_prediction(candidate, "lh-001-delayed-recall-10m")
        row["cited_observation_refs"] = []
        row["current_observation_refs"] = []
        score = self.score(candidate)
        result = self.episode_result(score, "lh-001-delayed-recall-10m")
        self.assertEqual(result["citation_recall"], 0.0)
        self.assertFalse(result["provenance_complete"])
        self.assertFalse(result["evidence_qualified_answer_correct"])
        self.assertLess(score["totals"]["citation_recall"], 1.0)

    def test_query_cutoff_must_match_declared_as_of_tick(self):
        candidate = copy.deepcopy(self.submission)
        self.episode_prediction(candidate, "lh-001-delayed-recall-10m")["evidence_as_of_tick"] = 599
        result = self.episode_result(self.score(candidate), "lh-001-delayed-recall-10m")
        self.assertIn("evidence_cutoff_must_equal_query_tick", result["violations"])

    def test_duplicate_event_identity_is_rejected(self):
        workload = copy.deepcopy(self.workload)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "lh-001-delayed-recall-10m")
        duplicate = copy.deepcopy(episode["stream_events"][0])
        episode["stream_events"].append(duplicate)
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)


    def test_duplicate_observation_reference_with_changed_digest_is_rejected(self):
        workload = copy.deepcopy(self.workload)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "lh-002-repeated-event-dedup")
        replay = next(x for x in episode["stream_events"] if x["event_id"] == "ev-tap-2-replay")
        replay["artifact_sha256"] = "f" * 64
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)

    def test_duplicate_observation_reference_with_changed_clock_generation_is_rejected(self):
        workload = copy.deepcopy(self.workload)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "lh-002-repeated-event-dedup")
        replay = next(x for x in episode["stream_events"] if x["event_id"] == "ev-tap-2-replay")
        replay["clock_generation"] = 1
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)

    def test_stream_delivery_order_must_be_monotonic(self):
        workload = copy.deepcopy(self.workload)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "lh-002-repeated-event-dedup")
        episode["stream_events"][1], episode["stream_events"][2] = (
            episode["stream_events"][2], episode["stream_events"][1]
        )
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)

    def test_derived_memory_unknown_source_dependency_is_rejected(self):
        workload = copy.deepcopy(self.workload)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "lh-008-memory-revocation")
        episode["derived_memory_items"][0]["derived_from_observation_refs"] = ["obs-not-in-stream"]
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)

    def test_derived_memory_dependency_cycle_is_rejected(self):
        workload = copy.deepcopy(self.workload)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "lh-008-memory-revocation")
        by_ref = {item["item_ref"]: item for item in episode["derived_memory_items"]}
        by_ref["mem-private-summary"]["derived_from_item_refs"] = ["mem-private-response-cache"]
        by_ref["mem-private-summary"]["derived_from_observation_refs"] = []
        by_ref["mem-private-working-context"]["created_tick"] = 600
        by_ref["mem-private-response-cache"]["created_tick"] = 600
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)

    def test_derived_memory_cannot_precede_source_observation(self):
        workload = copy.deepcopy(self.workload)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "lh-008-memory-revocation")
        private_source = next(
            event for event in episode["stream_events"]
            if event.get("observation_ref") == "obs-private-utterance"
        )
        private_source["availability_tick"] = 100
        episode["derived_memory_items"][0]["created_tick"] = 50
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload)

    def test_superseded_observation_is_allowed_as_history_but_not_current_state(self):
        score = self.score()
        result = self.episode_result(score, "lh-003-correction-supersession")
        self.assertEqual(result["violations"], [])
        self.assertTrue(result["provenance_complete"])

    def test_revoked_derived_memory_residue_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        row = self.episode_prediction(candidate, "lh-008-memory-revocation")
        row["retained_memory_item_refs"] = ["mem-private-embedding"]
        result = self.episode_result(self.score(candidate), "lh-008-memory-revocation")
        self.assertIn("revoked_evidence_or_derived_memory_reuse", result["violations"])
        self.assertFalse(result["evidence_compliant"])

    def test_malformed_retrieved_memory_reference_fails_closed(self):
        candidate = copy.deepcopy(self.submission)
        row = self.episode_prediction(candidate, "lh-008-memory-revocation")
        row["retrieved_memory_item_refs"] = [{}]
        result = self.episode_result(self.score(candidate), "lh-008-memory-revocation")
        self.assertIn("invalid_retrieved_memory_item_refs", result["violations"])

    def test_malformed_purge_receipt_status_fails_closed_without_evaluator_crash(self):
        candidate = copy.deepcopy(self.submission)
        row = self.episode_prediction(candidate, "lh-008-memory-revocation")
        row["purge_receipts"][0]["status"] = {}
        result = self.episode_result(self.score(candidate), "lh-008-memory-revocation")
        self.assertIn("purge_receipt_status_invalid", result["violations"])
        self.assertIn("revoked_evidence_or_derived_memory_reuse", result["violations"])

    def test_non_candidate_reported_purge_attestation_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        row = self.episode_prediction(candidate, "lh-008-memory-revocation")
        row["purge_receipts"][0]["attestation_level"] = "independently_verified"
        result = self.episode_result(self.score(candidate), "lh-008-memory-revocation")
        self.assertIn("purge_receipt_attestation_level_invalid", result["violations"])

    def test_revocation_requires_complete_transitive_purge_receipts(self):
        candidate = copy.deepcopy(self.submission)
        row = self.episode_prediction(candidate, "lh-008-memory-revocation")
        row["purge_receipts"] = [
            receipt for receipt in row["purge_receipts"]
            if receipt["item_ref"] != "mem-private-response-cache"
        ]
        result = self.episode_result(self.score(candidate), "lh-008-memory-revocation")
        self.assertIn("revoked_evidence_or_derived_memory_reuse", result["violations"])

    def test_duplicate_event_count_is_scored_separately_from_citation_count(self):
        candidate = copy.deepcopy(self.submission)
        row = self.episode_prediction(candidate, "lh-002-repeated-event-dedup")
        row["claimed_distinct_event_count"] = 5
        result = self.episode_result(self.score(candidate), "lh-002-repeated-event-dedup")
        self.assertFalse(result["answer_correct"])
        self.assertTrue(result["false_memory"])

    def test_truth_cannot_require_future_observation_at_query_cutoff(self):
        workload = copy.deepcopy(self.workload)
        truth = copy.deepcopy(self.truth)
        episode = next(x for x in workload["episodes"] if x["episode_id"] == "lh-001-delayed-recall-10m")
        episode["stream_events"].append({
            "event_id": "ev-future-truth-test", "kind": "observation", "availability_tick": 700,
            "observation_ref": "obs-future-truth-test", "modalities": ["video"], "capture_tick": 700,
            "artifact_sha256": "f" * 64, "clock_domain_id": "test-clock", "clock_generation": 0
        })
        label = next(x for x in truth["episodes"] if x["episode_id"] == "lh-001-delayed-recall-10m")
        label["required_observation_refs"] = ["obs-future-truth-test"]
        with self.assertRaises(evaluator.EvaluationError):
            self.score(workload=workload, truth=truth)

    def test_truth_cannot_require_revoked_observation(self):
        truth = copy.deepcopy(self.truth)
        label = next(x for x in truth["episodes"] if x["episode_id"] == "lh-008-memory-revocation")
        label["required_observation_refs"] = ["obs-private-utterance"]
        with self.assertRaises(evaluator.EvaluationError):
            self.score(truth=truth)


if __name__ == "__main__":
    unittest.main()