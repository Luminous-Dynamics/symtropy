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
                "lh-012-multisegment-15-source-chain": (
                    "ordered_15_span_chain", list(reversed(required)), False, False
                ),
            }
            if episode_id in choices:
                answer, citations, independent, cross_clock = choices[episode_id]
                abstained = False
        elif condition == "episodic_retrieval":
            if episode_id == "lh-003-correction-supersession":
                answer, citations, abstained = "initial_machine_state", ["obs-machine-state-v1"], False
            elif episode_id == "lh-008-memory-revocation":
                answer, citations, abstained = "private_utterance", ["obs-private-utterance"], False
            elif episode_id == "lh-012-multisegment-15-source-chain":
                answer, citations, abstained = "ordered_15_span_chain", required[:-1], False
        elif condition != "evidence_ledger":
            raise AssertionError(condition)

        if label["expected_decision"] == "abstain" and condition != "summary_only":
            answer, abstained, citations = None, True, []
        superseded = {
            event["supersedes_observation_ref"]
            for event in episode["stream_events"]
            if event["kind"] == "correction" and event["availability_tick"] <= query_tick
        }
        revoked = {
            event["target_observation_ref"]
            for event in episode["stream_events"]
            if event["kind"] == "revocation" and event["availability_tick"] <= query_tick
        }
        current_refs = (
            list(citations)
            if episode_id == "lh-003-correction-supersession" and answer == "initial_machine_state"
            else [ref for ref in citations if ref not in superseded and ref not in revoked]
        )
        retrieved_memory_refs = []
        retained_memory_refs = []
        purge_receipts = []
        if episode_id == "lh-008-memory-revocation":
            derived_items = episode.get("derived_memory_items", [])
            purge_status = {item["item_ref"]: "purged" for item in derived_items if item["item_ref"].startswith("mem-private-")}
            if condition == "summary_only":
                retrieved_memory_refs = ["mem-private-response-cache"]
                retained_memory_refs = ["mem-private-summary", "mem-private-response-cache"]
                purge_status["mem-private-summary"] = "retained"
                purge_status["mem-private-response-cache"] = "retained"
            elif condition == "episodic_retrieval":
                retrieved_memory_refs = ["mem-private-retrieval-index"]
                retained_memory_refs = ["mem-private-embedding", "mem-private-retrieval-index"]
                purge_status["mem-private-embedding"] = "retained"
                purge_status["mem-private-retrieval-index"] = "retained"
            elif condition == "evidence_ledger":
                retrieved_memory_refs = ["mem-public-action-summary"]
                retained_memory_refs = ["mem-public-action-summary"]
            purge_receipts = [
                {
                    "item_ref": item["item_ref"],
                    "status": purge_status.get(item["item_ref"], "retained"),
                    "purge_tick": 1800,
                    "receipt_id": f"test-{condition}-purge-{item['item_ref']}",
                    "attestation_level": "candidate_self_reported",
                }
                for item in derived_items if item["item_ref"].startswith("mem-private-")
            ]
        count_claim = None
        if answer == "four_distinct_taps":
            count_claim = 4
        elif answer == "five_distinct_taps":
            count_claim = 5
        predictions.append({
            "episode_id": episode_id,
            "decision": "abstain" if abstained else "answer",
            "answer": answer,
            "confidence": 0.0 if abstained else 0.9,
            "cited_observation_refs": citations,
            "current_observation_refs": current_refs,
            "claimed_independent_corroboration": independent,
            "claimed_cross_clock_order": cross_clock,
            "evidence_as_of_tick": query_tick,
            "response_tick": query_tick + 1,
            "claimed_distinct_event_count": count_claim,
            "retrieved_memory_item_refs": retrieved_memory_refs,
            "retained_memory_item_refs": retained_memory_refs,
            "purge_receipts": purge_receipts,
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
        "workload_ref": "git-blob-sha1:17dfedb940e2439c3e4e59c76e3ffc22f90ae917",
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
        self.assertEqual(ledger["answer_accuracy"], 11 / 12)
        self.assertEqual(ledger["provenance_complete_rate"], 1.0)
        self.assertEqual(report["per_condition"]["summary_only"]["answer_accuracy"], 4 / 12)
        self.assertLess(
            report["per_condition"]["summary_only"]["provenance_complete_rate"],
            ledger["provenance_complete_rate"],
        )

    def test_fifteen_span_chain_rewards_exact_sequence_not_unordered_recall(self):
        episode_id = "lh-012-multisegment-15-source-chain"
        results = {}
        for run in self.runs:
            score = evaluator.evaluate(self.workload, self.truth, run["submission"])
            results[run["memory_condition"]] = next(
                row for row in score["per_episode"] if row["episode_id"] == episode_id
            )
        self.assertTrue(results["evidence_ledger"]["answer_correct"])
        self.assertTrue(results["evidence_ledger"]["provenance_complete"])
        self.assertFalse(results["summary_only"]["answer_correct"])
        self.assertIn("current_observation_order_mismatch", results["summary_only"]["violations"])
        self.assertFalse(results["episodic_retrieval"]["answer_correct"])
        self.assertIn("current_observation_order_mismatch", results["episodic_retrieval"]["violations"])
        self.assertFalse(results["recent_window"]["answer_correct"])
        self.assertTrue(results["recent_window"]["abstained"])
        self.assertTrue(results["recent_window"]["evidence_compliant"])
        self.assertNotIn("current_observation_order_mismatch", results["recent_window"]["violations"])

    def test_future_correction_checkpoint_preserves_old_state_in_all_conditions(self):
        report = comparator.compare_memory_conditions(self.workload,self.truth,self.runs)
        self.assertEqual(report["per_condition"]["evidence_ledger"]["answer_accuracy"],11/12)
        for run in self.runs:
            prediction=next(x for x in run["submission"]["predictions"] if x["episode_id"]=="lh-010-correction-before-arrival")
            self.assertEqual(prediction["answer"],"initial_machine_state")
            self.assertEqual(prediction["current_observation_refs"],["obs-machine-state-v1"])

    def test_future_revocation_checkpoint_preserves_pre_revocation_eligibility(self):
        for run in self.runs:
            prediction=next(x for x in run["submission"]["predictions"] if x["episode_id"]=="lh-011-revocation-before-effect")
            self.assertEqual(prediction["answer"],"private_utterance_still_eligible")
            self.assertEqual(prediction["current_observation_refs"],["obs-private-utterance-pre-revoke"])

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

    def test_budget_caps_must_match_between_conditions(self):
        runs = copy.deepcopy(self.runs)
        runs[0]["budget"]["max_memory_bytes_per_episode"] = 65536
        with self.assertRaises(comparator.EvaluationError):
            comparator.compare_memory_conditions(self.workload, self.truth, runs)

    def test_wrong_exact_workload_blob_reference_is_rejected(self):
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