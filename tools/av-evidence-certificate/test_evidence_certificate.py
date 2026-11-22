import hashlib
import unittest

from evaluate_evidence_certificate import (
    BENCHMARK_ID,
    EvidenceEvaluationError,
    evaluate,
)


def digest(value):
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def workload():
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": 1,
        "episode_id": "episode-001",
        "transcript_head_sha256": digest("candidate-visible-transcript"),
        "decision_cutoff_tick": 10,
        "observation_catalog": [
            {
                "observation_id": "obs-video-01",
                "modality": "video",
                "available_tick": 5,
                "capture_complete_tick": 5,
                "capture_start_ms": 0,
                "capture_end_ms": 1000,
                "clock_domain_id": "camera-clock",
                "clock_generation": 2,
                "artifact_sha256": digest("video-bytes"),
            },
            {
                "observation_id": "obs-audio-01",
                "modality": "audio",
                "available_tick": 6,
                "capture_complete_tick": 6,
                "capture_start_ms": 0,
                "capture_end_ms": 1000,
                "clock_domain_id": "mic-clock",
                "clock_generation": 4,
                "artifact_sha256": digest("audio-bytes"),
            },
        ],
    }


def truth():
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": 1,
        "episode_id": "episode-001",
        "claims": [
            {
                "claim_id": "claim-door",
                "answerable": True,
                "acceptable_answer_values": ["door_opens"],
                "minimal_sufficient_evidence_sets": [["obs-video-01"]],
                "supporting_spans": [
                    {
                        "observation_ref": "obs-video-01",
                        "start_ms": 100,
                        "end_ms": 300,
                    }
                ],
                "supporting_boxes": [
                    {
                        "observation_ref": "obs-video-01",
                        "start_ms": 100,
                        "end_ms": 300,
                        "frame_id": "frame-12",
                        "bbox_xyxy": [0.1, 0.1, 0.5, 0.5],
                    }
                ],
                "temporal_iou_threshold": 0.3,
                "spatial_iou_threshold": 0.3,
            },
            {
                "claim_id": "claim-unknown",
                "answerable": False,
                "acceptable_answer_values": [],
                "minimal_sufficient_evidence_sets": [],
                "supporting_spans": [],
                "supporting_boxes": [],
            },
        ],
    }


def submission():
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": 1,
        "episode_id": "episode-001",
        "transcript_head_sha256": digest("candidate-visible-transcript"),
        "claims": [
            {
                "claim_id": "claim-door",
                "answer_value": "door_opens",
                "abstained": False,
                "evidence": [
                    {
                        "observation_ref": "obs-video-01",
                        "start_ms": 120,
                        "end_ms": 280,
                        "frame_id": "frame-12",
                        "bbox_xyxy": [0.1, 0.1, 0.5, 0.5],
                    }
                ],
            },
            {
                "claim_id": "claim-unknown",
                "answer_value": None,
                "abstained": True,
                "evidence": [],
            },
        ],
    }


class EvidenceCertificateTests(unittest.TestCase):
    def test_perfect_answer_and_grounding_are_scored_separately(self):
        result = evaluate(workload(), truth(), submission())
        self.assertEqual(result["answer_accuracy"], 1.0)
        self.assertEqual(result["grounded_answer_accuracy"], 1.0)
        self.assertEqual(result["unsupported_claim_rate"], 0.0)
        self.assertAlmostEqual(result["mean_temporal_iou"], 0.8)
        self.assertEqual(result["mean_spatial_iou"], 1.0)
        self.assertEqual(result["correct_abstention_count"], 1)

    def test_correct_answer_with_wrong_source_is_not_grounded(self):
        candidate = submission()
        candidate["claims"][0]["evidence"] = [
            {
                "observation_ref": "obs-audio-01",
                "start_ms": 120,
                "end_ms": 280,
            }
        ]
        result = evaluate(workload(), truth(), candidate)
        self.assertEqual(result["answer_accuracy"], 1.0)
        self.assertEqual(result["grounded_answer_accuracy"], 0.5)
        self.assertEqual(result["per_claim"][0]["citation_precision"], 0.0)
        self.assertFalse(result["per_claim"][0]["evidence_sufficient"])
        self.assertTrue(result["per_claim"][0]["unsupported_claim"])

    def test_candidate_must_cover_one_complete_minimal_evidence_set(self):
        hidden = truth()
        hidden["claims"][0]["minimal_sufficient_evidence_sets"] = [
            ["obs-video-01", "obs-audio-01"]
        ]
        candidate = submission()
        result = evaluate(workload(), hidden, candidate)
        claim = result["per_claim"][0]
        self.assertEqual(claim["citation_precision"], 1.0)
        self.assertEqual(claim["citation_recall"], 0.5)
        self.assertFalse(claim["evidence_sufficient"])
        self.assertFalse(claim["grounded_success"])

    def test_alternative_sufficient_evidence_sets_are_supported(self):
        hidden = truth()
        hidden["claims"][0]["minimal_sufficient_evidence_sets"] = [
            ["obs-video-01", "obs-audio-01"],
            ["obs-video-01"],
        ]
        result = evaluate(workload(), hidden, submission())
        self.assertTrue(result["per_claim"][0]["evidence_sufficient"])
        self.assertEqual(result["per_claim"][0]["citation_recall"], 1.0)

    def test_unnecessary_citations_are_reported(self):
        candidate = submission()
        candidate["claims"][0]["evidence"].append({
            "observation_ref": "obs-audio-01",
            "start_ms": 400,
            "end_ms": 500,
        })
        result = evaluate(workload(), truth(), candidate)
        self.assertEqual(result["per_claim"][0]["unnecessary_citations"], ["obs-audio-01"])
        self.assertTrue(result["per_claim"][0]["evidence_sufficient"])

    def test_wrong_answer_with_correct_citation_is_not_success(self):
        candidate = submission()
        candidate["claims"][0]["answer_value"] = "door_closes"
        result = evaluate(workload(), truth(), candidate)
        self.assertEqual(result["answer_accuracy"], 0.5)
        self.assertEqual(result["grounded_answer_accuracy"], 0.5)
        self.assertTrue(result["per_claim"][0]["evidence_sufficient"])
        self.assertFalse(result["per_claim"][0]["grounded_success"])

    def test_spatial_grounding_requires_matching_observation_and_frame(self):
        candidate = submission()
        candidate["claims"][0]["evidence"][0]["frame_id"] = "wrong-frame"
        result = evaluate(workload(), truth(), candidate)
        self.assertEqual(result["mean_spatial_iou"], 0.0)
        self.assertFalse(result["per_claim"][0]["spatial_iou_pass"])

    def test_temporal_grounding_uses_only_cited_capture_intervals(self):
        candidate = submission()
        candidate["claims"][0]["evidence"][0]["start_ms"] = 600
        candidate["claims"][0]["evidence"][0]["end_ms"] = 800
        result = evaluate(workload(), truth(), candidate)
        self.assertEqual(result["mean_temporal_iou"], 0.0)
        self.assertFalse(result["per_claim"][0]["temporal_iou_pass"])

    def test_unknown_observation_reference_is_rejected(self):
        candidate = submission()
        candidate["claims"][0]["evidence"][0]["observation_ref"] = "obs-hidden-future"
        with self.assertRaises(EvidenceEvaluationError):
            evaluate(workload(), truth(), candidate)

    def test_evidence_outside_observation_capture_interval_is_rejected(self):
        candidate = submission()
        candidate["claims"][0]["evidence"][0]["end_ms"] = 1200
        with self.assertRaises(EvidenceEvaluationError):
            evaluate(workload(), truth(), candidate)

    def test_observations_after_cutoff_are_rejected_from_workload(self):
        data = workload()
        data["observation_catalog"][0]["available_tick"] = 11
        with self.assertRaises(EvidenceEvaluationError):
            evaluate(data, truth(), submission())

    def test_capture_not_complete_by_cutoff_is_rejected(self):
        data = workload()
        data["observation_catalog"][0]["capture_complete_tick"] = 11
        with self.assertRaises(EvidenceEvaluationError):
            evaluate(data, truth(), submission())

    def test_transcript_head_mismatch_is_rejected(self):
        candidate = submission()
        candidate["transcript_head_sha256"] = digest("different-transcript")
        with self.assertRaises(EvidenceEvaluationError):
            evaluate(workload(), truth(), candidate)

    def test_duplicate_and_missing_candidate_claims_are_rejected(self):
        candidate = submission()
        candidate["claims"].append(dict(candidate["claims"][0]))
        with self.assertRaises(EvidenceEvaluationError):
            evaluate(workload(), truth(), candidate)
        candidate = submission()
        candidate["claims"].pop()
        with self.assertRaises(EvidenceEvaluationError):
            evaluate(workload(), truth(), candidate)

    def test_abstention_must_not_smuggle_answer_or_evidence(self):
        candidate = submission()
        candidate["claims"][0]["abstained"] = True
        with self.assertRaises(EvidenceEvaluationError):
            evaluate(workload(), truth(), candidate)

    def test_correct_abstention_is_scored_for_unanswerable_claim(self):
        candidate = submission()
        candidate["claims"][1]["abstained"] = True
        result = evaluate(workload(), truth(), candidate)
        self.assertTrue(result["per_claim"][1]["answer_correct"])
        self.assertTrue(result["per_claim"][1]["grounded_success"])

    def test_audio_observation_cannot_carry_spatial_video_box(self):
        candidate = submission()
        candidate["claims"][0]["evidence"] = [{
            "observation_ref": "obs-audio-01",
            "start_ms": 100,
            "end_ms": 300,
            "frame_id": "frame-audio",
            "bbox_xyxy": [0.1, 0.1, 0.5, 0.5],
        }]
        with self.assertRaises(EvidenceEvaluationError):
            evaluate(workload(), truth(), candidate)

    def test_unannotated_temporal_or_spatial_axes_are_not_imputed(self):
        hidden = truth()
        hidden["claims"][0]["supporting_spans"] = []
        hidden["claims"][0]["supporting_boxes"] = []
        result = evaluate(workload(), hidden, submission())
        self.assertIsNone(result["per_claim"][0]["temporal_iou"])
        self.assertIsNone(result["per_claim"][0]["spatial_iou"])
        self.assertIsNone(result["mean_temporal_iou"])
        self.assertIsNone(result["mean_spatial_iou"])

    def test_replay_is_deterministic(self):
        a = evaluate(workload(), truth(), submission())
        b = evaluate(workload(), truth(), submission())
        self.assertEqual(a, b)
        self.assertEqual(len(a["claim_scorecard_sha256"]), 64)


if __name__ == "__main__":
    unittest.main()
