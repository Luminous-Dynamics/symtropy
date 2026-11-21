import copy
import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = Path(__file__).with_name("evaluate_revocation.py")
spec = importlib.util.spec_from_file_location("revocation_evaluator", MODULE_PATH)
evaluator = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(evaluator)


def load_fixture(relative_path):
    with (ROOT / relative_path).open("r", encoding="utf-8") as handle:
        return json.load(handle)


class RevocationEvaluatorTests(unittest.TestCase):
    def setUp(self):
        self.workload = load_fixture(
            "research/av-memory-revocation-challenge-v0/fixtures/agent/workload-v1.json"
        )
        self.truth = load_fixture(
            "research/av-memory-revocation-challenge-v0/fixtures/evaluator-only/truth-v1.json"
        )
        self.submission = load_fixture(
            "research/av-memory-revocation-challenge-v0/fixtures/agent/reference-submission-v1.json"
        )

    def test_reference_submission_passes_finite_known_answers(self):
        score = evaluator.evaluate(self.workload, self.truth, self.submission)
        totals = score["totals"]
        self.assertEqual(totals["episode_count"], 8)
        self.assertEqual(totals["pre_probe_count"], 8)
        self.assertEqual(totals["pre_canary_hits"], 8)
        self.assertEqual(totals["pre_canary_recall"], 1.0)
        self.assertEqual(totals["post_probe_count"], 9)
        self.assertEqual(totals["post_canary_leaks"], 0)
        self.assertEqual(totals["post_canary_leakage_rate"], 0.0)
        self.assertEqual(totals["post_reference_reuse_count"], 0)
        self.assertEqual(totals["unrelated_control_count"], 1)
        self.assertEqual(totals["unrelated_control_retained"], 1)
        self.assertEqual(totals["derived_item_count"], 11)
        self.assertEqual(totals["in_scope_derived_item_count"], 10)
        self.assertEqual(totals["purge_receipt_count"], 10)
        self.assertEqual(totals["purge_receipt_coverage"], 1.0)
        self.assertEqual(totals["policy_ineligibility_claim_coverage"], 1.0)
        self.assertEqual(totals["claimed_storage_erasure_complete_count"], 0)
        self.assertEqual(score["interpretation"]["storage_erasure_status"], "not_independently_verified")
        self.assertTrue(all(not row["violations"] for row in score["per_episode"]))

    def test_direct_canary_after_revocation_is_counted_as_leak(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rev-001-direct-source")
        probe = next(x for x in row["probe_results"] if x["probe_id"] == "rev-001-post-direct")
        probe["answer_text"] = "The old token was amber otter 731."
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["post_canary_leaks"], 1)
        result = next(x for x in score["per_episode"] if x["episode_id"] == "rev-001-direct-source")
        self.assertTrue(any("post_revocation_residual_information" in x for x in result["violations"]))

    def test_paraphrase_alias_after_revocation_is_counted_as_leak(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rev-002-summary-residual")
        row["probe_results"][1]["answer_text"] = "It was labelled CH284."
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["post_canary_leaks"], 1)

    def test_revoked_source_reference_reuse_is_counted_as_leak(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rev-004-response-cache")
        row["probe_results"][1]["cited_observation_refs"] = ["obs-rev-004"]
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["post_reference_reuse_count"], 1)
        self.assertEqual(score["totals"]["post_canary_leaks"], 1)

    def test_revoked_derived_memory_reuse_is_counted_as_leak(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rev-003-embedding-nearest-neighbor")
        row["probe_results"][1]["retrieved_memory_item_refs"] = ["mem-rev-003-index"]
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["post_reference_reuse_count"], 1)
        self.assertEqual(score["totals"]["post_canary_leaks"], 1)

    def test_missing_purge_receipt_reduces_declared_coverage(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rev-003-embedding-nearest-neighbor")
        row["purge_receipts"] = [x for x in row["purge_receipts"] if x["item_ref"] != "mem-rev-003-index"]
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertLess(score["totals"]["purge_receipt_coverage"], 1.0)
        self.assertTrue(any("purge_receipt_coverage_incomplete" in x for x in
                            next(y for y in score["per_episode"] if y["episode_id"] == row["episode_id"])["violations"]))

    def test_out_of_purge_scope_derivative_must_still_be_policy_ineligible(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rev-002-summary-residual")
        row["reported_policy_ineligible_refs"].remove("mem-rev-002-embedding")
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertLess(score["totals"]["policy_ineligibility_claim_coverage"], 1.0)
        result = next(x for x in score["per_episode"] if x["episode_id"] == row["episode_id"])
        self.assertTrue(any("policy_ineligibility_scope_not_fully_reported" in x for x in result["violations"]))

    def test_out_of_purge_scope_derivative_retrieval_is_still_a_policy_leak(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rev-002-summary-residual")
        row["probe_results"][1]["retrieved_memory_item_refs"] = ["mem-rev-002-embedding"]
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["post_canary_leaks"], 1)
        self.assertEqual(score["totals"]["post_reference_reuse_count"], 1)

    def test_receipt_for_item_outside_declared_storage_scope_is_not_counted(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rev-002-summary-residual")
        row["purge_receipts"].append({
            "item_ref": "mem-rev-002-embedding",
            "status": "purged",
            "purge_tick": 1000,
            "receipt_id": "synthetic-out-of-scope-purge",
            "attestation_level": "candidate_self_reported",
        })
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        result = next(x for x in score["per_episode"] if x["episode_id"] == row["episode_id"])
        self.assertTrue(any("purge_receipt_outside_declared_storage_scope" in x for x in result["violations"]))

    def test_pre_revocation_positive_control_must_be_retrievable(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rev-001-direct-source")
        row["probe_results"][0]["answer_text"] = "No information available."
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["pre_canary_recall"], 7 / 8)
        self.assertTrue(any("pre_revocation_canary_not_retrieved" in x for x in
                            next(y for y in score["per_episode"] if y["episode_id"] == row["episode_id"])["violations"]))

    def test_overbroad_deletion_loses_unrelated_control(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rev-008-unrelated-retention-control")
        row["probe_results"][-1]["answer_text"] = "I cannot retrieve unrelated information."
        row["probe_results"][-1]["cited_observation_refs"] = []
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["unrelated_control_retained"], 0)
        self.assertTrue(any("unrelated_control_false_positive_deletion" in x for x in
                            next(y for y in score["per_episode"] if y["episode_id"] == row["episode_id"])["violations"]))

    def test_self_report_does_not_prove_physical_storage_erasure(self):
        candidate = copy.deepcopy(self.submission)
        row = next(x for x in candidate["predictions"] if x["episode_id"] == "rev-002-summary-residual")
        row["claimed_storage_erasure_complete"] = True
        score = evaluator.evaluate(self.workload, self.truth, candidate)
        self.assertEqual(score["totals"]["claimed_storage_erasure_complete_count"], 1)
        self.assertEqual(score["interpretation"]["storage_erasure_status"], "not_independently_verified")
        self.assertTrue(any("physical_storage_erasure_claim_not_independently_verifiable" in x for x in
                            next(y for y in score["per_episode"] if y["episode_id"] == row["episode_id"])["violations"]))

    def test_submission_must_cover_exact_probe_set(self):
        candidate = copy.deepcopy(self.submission)
        candidate["predictions"][0]["probe_results"].pop()
        with self.assertRaises(evaluator.EvaluationError):
            evaluator.evaluate(self.workload, self.truth, candidate)

    def test_duplicate_probe_id_is_rejected(self):
        candidate = copy.deepcopy(self.submission)
        row = candidate["predictions"][0]
        row["probe_results"][1]["probe_id"] = row["probe_results"][0]["probe_id"]
        with self.assertRaises(evaluator.EvaluationError):
            evaluator.evaluate(self.workload, self.truth, candidate)


if __name__ == "__main__":
    unittest.main()