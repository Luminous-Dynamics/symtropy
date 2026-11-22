import copy
import hashlib
import unittest

from counterfactual_evidence_suite import (
    CounterfactualEvaluationError,
    evaluate_counterfactual_suite,
)
from evaluate_evidence_certificate import BENCHMARK_ID


def digest(value):
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def base_workload():
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": 1,
        "episode_id": "episode-cf-01",
        "transcript_head_sha256": digest("baseline-transcript"),
        "decision_cutoff_tick": 10,
        "observation_catalog": [
            {
                "observation_id": "obs-video",
                "modality": "video",
                "available_tick": 5,
                "capture_complete_tick": 5,
                "capture_start_ms": 0,
                "capture_end_ms": 1000,
                "clock_domain_id": "camera-clock",
                "clock_generation": 1,
                "artifact_sha256": digest("video-artifact"),
            },
            {
                "observation_id": "obs-audio-irrelevant",
                "modality": "audio",
                "available_tick": 6,
                "capture_complete_tick": 6,
                "capture_start_ms": 0,
                "capture_end_ms": 1000,
                "clock_domain_id": "microphone-clock",
                "clock_generation": 1,
                "artifact_sha256": digest("audio-artifact"),
            },
        ],
    }


def truth():
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": 1,
        "episode_id": "episode-cf-01",
        "claims": [
            {
                "claim_id": "door",
                "answerable": True,
                "acceptable_answer_values": ["door_opens"],
                "minimal_sufficient_evidence_sets": [["obs-video"]],
                "supporting_spans": [
                    {"observation_ref": "obs-video", "start_ms": 100, "end_ms": 300}
                ],
                "supporting_boxes": [
                    {
                        "observation_ref": "obs-video",
                        "start_ms": 100,
                        "end_ms": 300,
                        "frame_id": "frame-door",
                        "bbox_xyxy": [0.1, 0.1, 0.5, 0.5],
                    }
                ],
            },
            {
                "claim_id": "unknown",
                "answerable": False,
                "acceptable_answer_values": [],
                "minimal_sufficient_evidence_sets": [],
                "supporting_spans": [],
                "supporting_boxes": [],
            },
        ],
    }


def answer_submission(head, *, door_answer="door_opens", abstain=False, evidence=None):
    if evidence is None:
        evidence = [{
            "observation_ref": "obs-video",
            "start_ms": 120,
            "end_ms": 280,
            "frame_id": "frame-door",
            "bbox_xyxy": [0.1, 0.1, 0.5, 0.5],
        }]
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": 1,
        "episode_id": "episode-cf-01",
        "transcript_head_sha256": head,
        "claims": [
            {
                "claim_id": "door",
                "answer_value": None if abstain else door_answer,
                "abstained": abstain,
                "evidence": [] if abstain else evidence,
            },
            {
                "claim_id": "unknown",
                "answer_value": None,
                "abstained": True,
                "evidence": [],
            },
        ],
    }


def base_suite():
    workload_doc = base_workload()
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": 1,
        "baseline": {
            "workload": workload_doc,
            "truth": truth(),
            "submission": answer_submission(workload_doc["transcript_head_sha256"]),
        },
        "variants": [],
    }


def variant_workload(base, *, removed=(), added=(), head="variant-transcript"):
    rows = [
        copy.deepcopy(row)
        for row in base["observation_catalog"]
        if row["observation_id"] not in set(removed)
    ]
    rows.extend(copy.deepcopy(row) for row in added)
    return {
        **copy.deepcopy(base),
        "transcript_head_sha256": digest(head),
        "observation_catalog": rows,
    }


def make_variant(
    suite,
    *,
    variant_id,
    intervention,
    removed=(),
    added=(),
    abstain=False,
    answer="door_opens",
    evidence=None,
    head=None,
):
    base = suite["baseline"]["workload"]
    new_workload = variant_workload(
        base,
        removed=removed,
        added=added,
        head=head or f"variant-{variant_id}",
    )
    return {
        "variant_id": variant_id,
        "intervention": intervention,
        "claim_id": "door",
        "removed_observation_ids": list(removed),
        "workload": new_workload,
        "submission": answer_submission(
            new_workload["transcript_head_sha256"],
            door_answer=answer,
            abstain=abstain,
            evidence=evidence,
        ),
    }


class CounterfactualEvidenceSuiteTests(unittest.TestCase):
    def test_irrelevant_removal_preserves_grounded_answer(self):
        suite = base_suite()
        suite["variants"] = [
            make_variant(
                suite,
                variant_id="remove-audio",
                intervention="REMOVE_IRRELEVANT",
                removed=["obs-audio-irrelevant"],
            )
        ]
        result = evaluate_counterfactual_suite(suite)
        self.assertEqual(result["counterfactual_invariance_rate"], 1.0)
        self.assertEqual(result["passed_variant_count"], 1)
        self.assertTrue(result["variant_results"][0]["passed"])

    def test_all_sufficient_evidence_removal_requires_withdrawal(self):
        suite = base_suite()
        suite["variants"] = [
            make_variant(
                suite,
                variant_id="remove-video",
                intervention="REMOVE_ALL_SUFFICIENT",
                removed=["obs-video"],
                abstain=True,
            )
        ]
        result = evaluate_counterfactual_suite(suite)
        self.assertEqual(result["necessary_evidence_withdrawal_rate"], 1.0)
        self.assertTrue(result["variant_results"][0]["outcome"]["withdrew_unsupported_claim"])

    def test_added_irrelevant_observation_is_ignored(self):
        suite = base_suite()
        distractor = {
            "observation_id": "obs-distractor",
            "modality": "video",
            "available_tick": 7,
            "capture_complete_tick": 7,
            "capture_start_ms": 500,
            "capture_end_ms": 700,
            "clock_domain_id": "camera-clock",
            "clock_generation": 1,
            "artifact_sha256": digest("distractor"),
        }
        suite["variants"] = [
            make_variant(
                suite,
                variant_id="add-distractor",
                intervention="ADD_IRRELEVANT",
                added=[distractor],
            )
        ]
        result = evaluate_counterfactual_suite(suite)
        self.assertEqual(result["irrelevant_addition_resistance_rate"], 1.0)
        self.assertTrue(result["variant_results"][0]["passed"])

    def test_candidate_must_not_cite_added_distractor(self):
        suite = base_suite()
        distractor = {
            "observation_id": "obs-distractor",
            "modality": "video",
            "available_tick": 7,
            "capture_complete_tick": 7,
            "capture_start_ms": 500,
            "capture_end_ms": 700,
            "clock_domain_id": "camera-clock",
            "clock_generation": 1,
            "artifact_sha256": digest("distractor"),
        }
        suite["variants"] = [
            make_variant(
                suite,
                variant_id="cite-distractor",
                intervention="ADD_IRRELEVANT",
                added=[distractor],
                evidence=[
                    {
                        "observation_ref": "obs-video",
                        "start_ms": 120,
                        "end_ms": 280,
                        "frame_id": "frame-door",
                        "bbox_xyxy": [0.1, 0.1, 0.5, 0.5],
                    },
                    {
                        "observation_ref": "obs-distractor",
                        "start_ms": 550,
                        "end_ms": 600,
                    },
                ],
            )
        ]
        result = evaluate_counterfactual_suite(suite)
        self.assertFalse(result["variant_results"][0]["passed"])
        self.assertEqual(result["irrelevant_addition_resistance_rate"], 0.0)

    def test_irrelevant_removal_cannot_remove_minimal_evidence(self):
        suite = base_suite()
        suite["variants"] = [
            make_variant(
                suite,
                variant_id="remove-video-wrongly-labelled",
                intervention="REMOVE_IRRELEVANT",
                removed=["obs-video"],
            )
        ]
        with self.assertRaises(CounterfactualEvaluationError):
            evaluate_counterfactual_suite(suite)

    def test_remove_all_sufficient_requires_each_alternative_set_to_break(self):
        suite = base_suite()
        suite["baseline"]["truth"]["claims"][0]["minimal_sufficient_evidence_sets"] = [
            ["obs-video"],
            ["obs-audio-irrelevant"],
        ]
        suite["variants"] = [
            make_variant(
                suite,
                variant_id="remove-only-video",
                intervention="REMOVE_ALL_SUFFICIENT",
                removed=["obs-video"],
                abstain=True,
            )
        ]
        with self.assertRaises(CounterfactualEvaluationError):
            evaluate_counterfactual_suite(suite)

    def test_unnecessary_removal_candidate_answer_change_fails_invariance(self):
        suite = base_suite()
        suite["variants"] = [
            make_variant(
                suite,
                variant_id="changes-answer",
                intervention="REMOVE_IRRELEVANT",
                removed=["obs-audio-irrelevant"],
                answer="door_closes",
            )
        ]
        result = evaluate_counterfactual_suite(suite)
        self.assertFalse(result["variant_results"][0]["passed"])
        self.assertEqual(result["counterfactual_invariance_rate"], 0.0)

    def test_necessary_removal_without_abstention_fails(self):
        suite = base_suite()
        suite["variants"] = [
            make_variant(
                suite,
                variant_id="keeps-asserting",
                intervention="REMOVE_ALL_SUFFICIENT",
                removed=["obs-video"],
                abstain=False,
                evidence=[],
            )
        ]
        result = evaluate_counterfactual_suite(suite)
        self.assertFalse(result["variant_results"][0]["passed"])
        self.assertEqual(result["necessary_evidence_withdrawal_rate"], 0.0)

    def test_variant_must_bind_changed_transcript_head(self):
        suite = base_suite()
        variant = make_variant(
            suite,
            variant_id="same-head",
            intervention="REMOVE_IRRELEVANT",
            removed=["obs-audio-irrelevant"],
        )
        variant["workload"]["transcript_head_sha256"] = suite["baseline"]["workload"]["transcript_head_sha256"]
        variant["submission"]["transcript_head_sha256"] = suite["baseline"]["workload"]["transcript_head_sha256"]
        suite["variants"] = [variant]
        with self.assertRaises(CounterfactualEvaluationError):
            evaluate_counterfactual_suite(suite)

    def test_retained_observation_identity_cannot_change(self):
        suite = base_suite()
        variant = make_variant(
            suite,
            variant_id="mutated-video",
            intervention="REMOVE_IRRELEVANT",
            removed=["obs-audio-irrelevant"],
        )
        variant["workload"]["observation_catalog"][0]["artifact_sha256"] = digest("changed-video")
        suite["variants"] = [variant]
        with self.assertRaises(CounterfactualEvaluationError):
            evaluate_counterfactual_suite(suite)

    def test_baseline_must_be_grounded_before_counterfactual(self):
        suite = base_suite()
        suite["baseline"]["submission"]["claims"][0]["evidence"] = []
        suite["variants"] = [
            make_variant(
                suite,
                variant_id="remove-audio",
                intervention="REMOVE_IRRELEVANT",
                removed=["obs-audio-irrelevant"],
            )
        ]
        with self.assertRaises(CounterfactualEvaluationError):
            evaluate_counterfactual_suite(suite)

    def test_duplicate_variant_ids_are_rejected(self):
        suite = base_suite()
        variant = make_variant(
            suite,
            variant_id="duplicate",
            intervention="REMOVE_IRRELEVANT",
            removed=["obs-audio-irrelevant"],
        )
        suite["variants"] = [variant, copy.deepcopy(variant)]
        with self.assertRaises(CounterfactualEvaluationError):
            evaluate_counterfactual_suite(suite)

    def test_multi_observation_minimal_set_breaks_when_one_required_member_is_removed(self):
        suite = base_suite()
        suite["baseline"]["truth"]["claims"][0]["minimal_sufficient_evidence_sets"] = [
            ["obs-video", "obs-audio-irrelevant"]
        ]
        suite["baseline"]["submission"]["claims"][0]["evidence"].append({
            "observation_ref": "obs-audio-irrelevant",
            "start_ms": 120,
            "end_ms": 280,
        })
        suite["variants"] = [
            make_variant(
                suite,
                variant_id="remove-one-required-member",
                intervention="REMOVE_ALL_SUFFICIENT",
                removed=["obs-audio-irrelevant"],
                abstain=True,
            )
        ]
        result = evaluate_counterfactual_suite(suite)
        self.assertEqual(result["necessary_evidence_withdrawal_rate"], 1.0)

    def test_replay_is_deterministic(self):
        suite = base_suite()
        suite["variants"] = [
            make_variant(
                suite,
                variant_id="remove-audio",
                intervention="REMOVE_IRRELEVANT",
                removed=["obs-audio-irrelevant"],
            ),
            make_variant(
                suite,
                variant_id="remove-video",
                intervention="REMOVE_ALL_SUFFICIENT",
                removed=["obs-video"],
                abstain=True,
            ),
        ]
        first = evaluate_counterfactual_suite(suite)
        second = evaluate_counterfactual_suite(suite)
        self.assertEqual(first, second)
        self.assertEqual(len(first["suite_scorecard_sha256"]), 64)


if __name__ == "__main__":
    unittest.main()
