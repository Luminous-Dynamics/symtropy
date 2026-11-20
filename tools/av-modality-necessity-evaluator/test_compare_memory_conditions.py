import copy
import importlib.util
import unittest
from pathlib import Path

import long_horizon_evaluate as evaluator

MODULE_PATH = Path(__file__).with_name("compare_memory_conditions.py")
spec = importlib.util.spec_from_file_location("memory_comparator", MODULE_PATH)
comparator = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(comparator)

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "research" / "av-long-horizon-evidence-v0" / "fixtures"


def build_run(workload, truth, condition, run_id):
    truth_by_id = {row["episode_id"]: row for row in truth["episodes"]}
    predictions = []
    for episode in workload["episodes"]:
        episode_id = episode["episode_id"]
        label = truth_by_id[episode_id]
        query_tick = episode["query"]["query_tick"]
        required = label["required_observation_refs"]
        all_observations = {
            event["observation_ref"]: event["availability_tick"]
            for event in episode["stream_events"] if event["kind"] == "observation"
        }
        available = set(all_observations)
        answer = label["expected_answer"]
        abstained = label["expected_decision"] == "abstain"
        citations = list(required)
        independent = False
        cross_clock = False

        if condition == "recent_window":
            retained = {ref for ref in required if query_tick - all_observations[ref] <= 1200}
            can_answer = label["expected_decision"] == "abstain" or retained == set(required)
            if not can_answer:
                answer, abstained, citations = None, True, []
            else:
                citations = list(required)
        elif condition == "summary_only":
            choices = {
                "lh-002-repeated-event-dedup": ("five_distinct_taps", required, False, False),
                "lh-003-correction-supersession": ("initial_machine_state", ["obs-machine-state-v1"], False, False),
                "lh-004-common-cause-dependence": ("independent_views", required, True, False),
                "lh-007-cross-clock-and-event-order": ("single_global_order", required, False, True),
                "lh-008-memory-revocation": ("private_utterance_still_eligible", ["obs-private-utterance"], False, False),
                "lh-009-compression-retention": ("routine_noise_6", ["obs-routine-noise-6"], False, False),
            }
            if episode_id in choices:
                answer, citations, independent, cross_clock = choices[episode_id]
                abstained = False
        elif condition == "episodic_retrieval":
            if episode_id == "lh-003-correction-supersession":
                answer, citations, abstained = "initial_machine_state", ["obs-machine-state-v1"], False
            elif episode_id == "lh-008-memory-revocation":
                answer, citations, abstained = "private_utterance", ["obs-private-utterance"], False
        elif condition != "evidence_ledger":
            raise AssertionError(condition)

        if label["expected_decision"] == "abstain" and condition != "summary_only":
            answer, abstained, citations = None, True, []
        predictions.append({
            "episode_id": episode_id,
            "answer": answer,
            "abstained": abstained,
            "confidence": 0.0 if abstained else 0.9,
            "cited_observation_refs": citations,
            "evidence_as_of_tick": query_tick,
            "response_tick": query_tick + 1,
            "claimed_independent_corroboration": independent,
            "claimed_cross_clock_ordering": cross_clock,
            "memory_bytes_used": None,
            "input_tokens": None,
            "retrieval_latency_ms": None,
            "energy_joules": None,
        })
    submission = {
        "schema_version": 1, "benchmark_id": "av-long-horizon-evidence-v0",
        "policy_id": "matched-memory-reference-v1", "run_id": run_id,
        "memory_condition": condition, "model_revision": None, "runtime_revision": "synthetic-v1",
        "predictions": predictions,
    }
    return {
        "schema_version": 1, "benchmark_id": "av-long-horizon-evidence-v0",
        "comparison_id": "synthetic-memory-tournament-v1",
        "policy_id": "matched-memory-reference-v1", "run_id": run_id,
        "memory_condition": condition, "model_revision": None, "runtime_revision": "synthetic-v1",
        "hardware_id": "synthetic-no-device", "seed": 0,
        "workload_ref": "git-blob-sha1:1a3b5e50b22ee8e65d894243ddcae7381e3f4afc",
        "protocol_id": "long-horizon-memory-ablation-v1",
        "memory_budget_profile_id": "workload-per-episode-budget-v1",
        "measurement_status": "synthetic_reference",
        "budget": {
            "max_memory_bytes_per_episode": 262144,
            "max_input_tokens_per_episode": 8192,
            "max_retrieval_latency_ms": 5000,
            "max_energy_joules_per_episode": 100
        },
        "submission": submission,
    }


class MemoryTournamentTests(unittest.TestCase):
    def setUp(self):
        self.workload = evaluator.read_json(BASE / "agent" / "workload-v1.json")
        self.truth = evaluator.read_json(BASE / "evaluator-only" / "truth-v1.json")
        self.runs = [
            build_run(self.workload, self.truth, condition, condition + "-run")
            for condition in ("recent_window", "summary_only", "episodic_retrieval", "evidence_ledger")
        ]

    def test_four_way_tournament_distinguishes_answer_and_provenance(self):
        report = comparator.compare_memory_conditions(self.workload, self.truth, self.runs)
        self.assertEqual(set(report["per_condition"]), {
            "recent_window", "summary_only", "episodic_retrieval", "evidence_ledger"
        })
        ledger = report["per_condition"]["evidence_ledger"]
        self.assertEqual(ledger["answer_accuracy"], 8 / 9)
        self.assertEqual(ledger["provenance_complete_rate"], 1.0)
        self.assertEqual(report["per_condition"]["summary_only"]["answer_accuracy"], 2 / 9)
        self.assertLess(
            report["per_condition"]["summary_only"]["provenance_complete_rate"],
            ledger["provenance_complete_rate"],
        )

    def test_mismatched_budget_profile_is_rejected(self):
        runs = copy.deepcopy(self.runs)
        runs[0]["memory_budget_profile_id"] = "unmatched-budget"
        with self.assertRaises(comparator.EvaluationError):
            comparator.compare_memory_conditions(self.workload, self.truth, runs)

    def test_exact_workload_blob_reference_is_required_by_cli_path(self):
        with self.assertRaises(comparator.EvaluationError):
            comparator.compare_memory_conditions(
                self.workload, self.truth, self.runs,
                workload_ref="git-blob-sha1:" + "0" * 40,
            )

    def test_synthetic_run_cannot_claim_memory_or_latency_measurements(self):
        runs = copy.deepcopy(self.runs)
        runs[0]["submission"]["predictions"][0]["memory_bytes_used"] = 4
        with self.assertRaises(comparator.EvaluationError):
            comparator.compare_memory_conditions(self.workload, self.truth, runs)

    def test_nested_submission_condition_must_match_outer_run(self):
        runs = copy.deepcopy(self.runs)
        runs[2]["submission"]["memory_condition"] = "summary_only"
        with self.assertRaises(comparator.EvaluationError):
            comparator.compare_memory_conditions(self.workload, self.truth, runs)


if __name__ == "__main__":
    unittest.main()