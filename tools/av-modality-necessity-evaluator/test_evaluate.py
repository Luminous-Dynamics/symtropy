import copy
import importlib.util
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("evaluate.py")
spec = importlib.util.spec_from_file_location("av_evaluator", MODULE_PATH)
evaluator = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(evaluator)


def make_episode(episode_id, necessity, answer, modalities, availability=10,
                 pair_id=None, shared_group=False, audio=True, transcript_available=True):
    video = {
        "observation_ref": f"v-{episode_id}",
        "source_dependency_group": f"shared-{episode_id}" if shared_group else f"video-{episode_id}",
    }
    audio_row = None if not audio else {
        "observation_ref": f"a-{episode_id}",
        "source_dependency_group": f"shared-{episode_id}" if shared_group else f"audio-{episode_id}",
    }
    episode = {
        "episode_id": episode_id,
        "task": {"deadline_tick": 50, "query_cutoff_tick": 60},
        "video": video, "audio": audio_row,
        "availability": {"video_tick": availability, "audio_tick": availability if audio else None},
    }
    if not transcript_available:
        episode["audio_profile"] = {"transcript_available": False}
    if pair_id:
        episode["pair_id"] = pair_id
    truth = {"episode_id": episode_id, "necessity": necessity, "truth_answer": answer}
    used = [m for m in ("audio", "video") if m in modalities]
    refs = [f"{'a' if m == 'audio' else 'v'}-{episode_id}" for m in used]
    prediction = {
        "episode_id": episode_id, "answer": answer, "confidence": 0.9, "abstained": False,
        "used_modalities": used, "used_transcript": False, "cited_observation_refs": refs,
        "claimed_independent_corroboration": False, "claimed_cross_modal_association": False,
        "evidence_as_of_tick": availability, "response_tick": availability + 1,
    }
    if necessity == "abstain_audio_missing":
        truth["truth_answer"] = "insufficient_audio_evidence"
        prediction.update(answer=None, confidence=0.0, abstained=True, used_modalities=["video"],
                          cited_observation_refs=[f"v-{episode_id}"])
    if shared_group:
        truth["shared_dependency_group"] = f"shared-{episode_id}"
    return episode, truth, prediction


def make_inputs():
    cases = [
        make_episode("audio-a", "audio_necessary", "impact", {"audio"}, pair_id="audio-pair"),
        make_episode("audio-b", "audio_necessary", "footsteps", {"audio"}, pair_id="audio-pair"),
        make_episode("common", "dependent_redundant_views", "one_source", {"audio", "video"}, shared_group=True),
        make_episode("misaligned", "no_cross_modal_association", "not_coincident", {"audio", "video"}),
        make_episode("conflict", "conflict_abstain_or_separate_hypotheses", "different_sources", {"audio", "video"}),
        make_episode("missing", "abstain_audio_missing", "unused", {"video"}, audio=False),
        make_episode("nonspeech", "non_speech_audio_required", "alarm_pulse", {"audio"}, transcript_available=False),
    ]
    return (
        {"schema_version": 1, "benchmark_id": "av-modality-necessity-v0", "episodes": [x[0] for x in cases]},
        {"schema_version": 1, "benchmark_id": "av-modality-necessity-v0", "episodes": [x[1] for x in cases]},
        {"schema_version": 1, "benchmark_id": "av-modality-necessity-v0", "policy_id": "test-policy",
         "run_id": "test-run", "predictions": [x[2] for x in cases]},
    )


class EvaluatorTests(unittest.TestCase):
    def setUp(self):
        self.workload, self.truth, self.submission = make_inputs()

    def test_reference_policy_is_fully_correct(self):
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        self.assertEqual(score["totals"]["episode_count"], 7)
        self.assertEqual(score["totals"]["decision_correct_count"], 7)
        self.assertEqual(score["totals"]["hard_violation_count"], 0)
        self.assertTrue(score["paired_groups"][0]["counterfactual_discrimination_observed"])

    def test_future_observation_is_a_hard_violation(self):
        candidate = copy.deepcopy(self.submission)
        candidate["predictions"][0]["evidence_as_of_tick"] = 9
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertIn("audio_not_available_at_evidence_cutoff", score["hard_violations"][0]["violations"])

    def test_common_cause_cannot_be_claimed_as_independent(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "common")
        row["claimed_independent_corroboration"] = True
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertTrue(any("common_cause_views_claimed_as_independent_corroboration" in x["violations"] for x in score["hard_violations"]))

    def test_misaligned_events_cannot_be_associated(self):
        candidate = copy.deepcopy(self.submission)
        next(x for x in candidate["predictions"] if x["episode_id"] == "misaligned")["claimed_cross_modal_association"] = True
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertTrue(any("cross_modal_association_claimed_under_conflict_or_misalignment" in x["violations"] for x in score["hard_violations"]))

    def test_missing_audio_requires_abstention(self):
        candidate = copy.deepcopy(self.submission)
        next(x for x in candidate["predictions"] if x["episode_id"] == "missing").update(
            abstained=False, answer="alarm_pulse", confidence=0.8)
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertTrue(any("missing_necessary_audio_requires_abstention" in x["violations"] for x in score["hard_violations"]))

    def test_non_speech_task_rejects_transcript_shortcut(self):
        candidate = copy.deepcopy(self.submission)
        next(x for x in candidate["predictions"] if x["episode_id"] == "nonspeech")["used_transcript"] = True
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertTrue(any("transcript_used_when_unavailable" in x["violations"] for x in score["hard_violations"]))

    def test_omitting_required_audio_breaks_evidence_contract(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "audio-a")
        row["used_modalities"], row["cited_observation_refs"] = [], []
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertTrue(any("necessary_modality_not_declared_as_used" in x["violations"] for x in score["hard_violations"]))


    def test_conflicting_sources_may_be_safely_abstained(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "conflict")
        row.update(answer=None, confidence=0.0, abstained=True)
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        result = next(x for x in score["per_episode"] if x["episode_id"] == "conflict")
        self.assertTrue(result["decision_correct"])
        self.assertFalse(result["answer_correct"])
        self.assertTrue(result["evidence_compliant"])

    def test_misaligned_streams_may_be_safely_abstained(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "misaligned")
        row.update(answer=None, confidence=0.0, abstained=True, claimed_cross_modal_association=False)
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        result = next(x for x in score["per_episode"] if x["episode_id"] == "misaligned")
        self.assertTrue(result["decision_correct"])
        self.assertEqual(result["violations"], [])


    def test_missing_necessary_modality_is_safe_abstention_not_answer(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "audio-a")
        row.update(answer=None, confidence=0.0, abstained=True,
                   used_modalities=["video"], cited_observation_refs=["v-audio-a"])
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        result = next(x for x in score["per_episode"] if x["episode_id"] == "audio-a")
        self.assertTrue(result["decision_correct"])
        self.assertFalse(result["answer_correct"])
        self.assertTrue(result["evidence_compliant"])

    def test_conflict_answer_requires_both_modalities(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "conflict")
        row.update(used_modalities=["audio"], cited_observation_refs=["a-conflict"])
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertTrue(any(
            "cross_modal_task_answered_without_both_modalities" in item["violations"]
            for item in score["hard_violations"]
        ))

    def test_submission_must_cover_every_episode_once(self):
        candidate = copy.deepcopy(self.submission)
        candidate["predictions"].pop()
        with self.assertRaises(evaluator.EvaluationError):
            evaluator.evaluate(self.workload, self.truth, candidate)


if __name__ == "__main__":
    unittest.main()