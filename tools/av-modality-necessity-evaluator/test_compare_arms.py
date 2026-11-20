import copy
import importlib.util
import unittest
from pathlib import Path

from test_evaluate import make_inputs

MODULE_PATH = Path(__file__).with_name("compare_arms.py")
spec = importlib.util.spec_from_file_location("arm_comparator", MODULE_PATH)
comparator = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(comparator)


def build_run(workload, truth, arm, run_id):
    allowed = comparator.EXPECTED_ARMS[arm]
    truth_by_id = {row["episode_id"]: row for row in truth["episodes"]}
    predictions = []
    for episode in workload["episodes"]:
        identifier = episode["episode_id"]
        label = truth_by_id[identifier]
        availability = episode.get("availability") or {}
        used = []
        refs = []
        for modality in ("audio", "video"):
            observation = episode.get(modality)
            if modality in allowed and isinstance(observation, dict):
                used.append(modality)
                refs.append(observation["observation_ref"])
        # Use the same frozen necessity table as the production evaluator.
        necessary = comparator.NECESSARY_MODALITIES.get(label["necessity"], set())
        can_answer = necessary <= set(used)
        if label["necessity"] == "abstain_audio_missing":
            can_answer = False
        if label["necessity"] in {
            "conflict_abstain_or_separate_hypotheses",
            "no_cross_modal_association",
            "dependent_redundant_views",
        }:
            can_answer = {"audio", "video"} <= set(used)
        answer = label["truth_answer"] if can_answer else None
        abstained = not can_answer
        ticks = [
            availability.get("audio_tick") if modality == "audio" else availability.get("video_tick")
            for modality in used
        ]
        ticks = [tick for tick in ticks if isinstance(tick, int)]
        evidence_tick = max(ticks, default=0)
        predictions.append({
            "episode_id": identifier,
            "answer": answer,
            "confidence": 0.95 if can_answer else 0.0,
            "abstained": abstained,
            "used_modalities": used,
            "used_transcript": False,
            "cited_observation_refs": refs,
            "claimed_independent_corroboration": False,
            "claimed_cross_modal_association": False,
            "evidence_as_of_tick": evidence_tick,
            "response_tick": evidence_tick + 1,
        })
    submission = {
        "schema_version": 1,
        "benchmark_id": "av-modality-necessity-v0",
        "policy_id": "synthetic-reference-policy",
        "run_id": run_id,
        "model_revision": None,
        "runtime_revision": "reference-v1",
        "predictions": predictions,
    }
    return {
        "schema_version": 1,
        "benchmark_id": "av-modality-necessity-v0",
        "comparison_id": "matched-reference-ablation-v1",
        "policy_id": "synthetic-reference-policy",
        "run_id": run_id,
        "input_arm": arm,
        "allowed_modalities": sorted(allowed),
        "model_revision": None,
        "runtime_revision": "reference-v1",
        "hardware_id": "synthetic-no-device",
        "seed": 0,
        "workload_ref": "git-blob-sha1:c75da05798b0d100bae2366e9fade87056e6e4d3",
        "protocol_id": "counterfactual-av-ablation-v1",
        "shared_config_id": "synthetic-reference-rule-policy-v1",
        "budget": {
            "max_audio_bytes": 1000000,
            "max_video_frames": 32,
            "max_input_tokens": 8192,
            "max_output_tokens": 512,
            "max_response_ms": 1000,
            "max_energy_joules": 100
        },
        "submission": submission,
    }


class ArmTournamentTests(unittest.TestCase):
    def setUp(self):
        self.workload, self.truth, _ = make_inputs()
        self.runs = [
            build_run(self.workload, self.truth, "video_only", "video-run"),
            build_run(self.workload, self.truth, "audio_only", "audio-run"),
            build_run(self.workload, self.truth, "audio_video", "av-run"),
        ]

    def test_matched_tournament_reports_expected_gains(self):
        report = comparator.compare_arms(self.workload, self.truth, self.runs)
        self.assertEqual(set(report["per_arm"]), {"video_only", "audio_only", "audio_video"})
        self.assertGreaterEqual(
            report["paired_answer_comparisons"]["audio_video_vs_video_only"]["overall"]["candidate_answer_wins"], 1
        )
        self.assertGreaterEqual(
            report["paired_answer_comparisons"]["audio_video_vs_audio_only"]["overall"]["candidate_answer_wins"], 1
        )

    def test_mismatched_hardware_is_rejected(self):
        runs = copy.deepcopy(self.runs)
        runs[0]["hardware_id"] = "different-hardware"
        with self.assertRaises(comparator.EvaluationError):
            comparator.compare_arms(self.workload, self.truth, runs)

    def test_declared_arm_cannot_use_unavailable_modality(self):
        runs = copy.deepcopy(self.runs)
        prediction = runs[0]["submission"]["predictions"][0]
        prediction["used_modalities"] = ["audio"]
        prediction["cited_observation_refs"] = [
            next(row["audio"]["observation_ref"] for row in self.workload["episodes"] if row["episode_id"] == prediction["episode_id"])
        ]
        with self.assertRaises(comparator.EvaluationError):
            comparator.compare_arms(self.workload, self.truth, runs)


    def test_wrong_exact_workload_blob_reference_is_rejected(self):
        with self.assertRaises(comparator.EvaluationError):
            comparator.compare_arms(
                self.workload, self.truth, self.runs,
                workload_ref="git-blob-sha1:" + "0" * 40,
            )

    def test_nested_model_revision_must_match_arm_envelope(self):
        runs = copy.deepcopy(self.runs)
        runs[2]["submission"]["runtime_revision"] = "different-runtime"
        with self.assertRaises(comparator.EvaluationError):
            comparator.compare_arms(self.workload, self.truth, runs)

    def test_transcript_path_is_not_silently_counted_as_raw_audio(self):
        runs = copy.deepcopy(self.runs)
        runs[1]["submission"]["predictions"][0]["used_transcript"] = True
        with self.assertRaises(comparator.EvaluationError):
            comparator.compare_arms(self.workload, self.truth, runs)

    def test_exact_p_value_is_bounded_and_symmetric(self):
        self.assertEqual(comparator._exact_binomial_two_sided(0, 0), 1.0)
        self.assertEqual(comparator._exact_binomial_two_sided(4, 0), 0.125)
        self.assertEqual(comparator._exact_binomial_two_sided(0, 4), 0.125)


if __name__ == "__main__":
    unittest.main()