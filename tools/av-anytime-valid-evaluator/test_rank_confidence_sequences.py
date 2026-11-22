import hashlib
import unittest

from rank_confidence_sequences import (
    BENCHMARK_ID,
    RankEvaluationError,
    evaluate_document,
    make_order_commitment,
    weak_orders,
)


def make_document(
    arms=("A0", "A1", "A2"),
    scores=None,
    *,
    alpha=0.05,
    lambdas=(0.03, 0.06, 0.12, 0.25, 0.5),
):
    episode_ids = [f"episode-{index:04d}" for index in range(30)]
    episode_order = list(reversed(episode_ids))
    seed_commitment = hashlib.sha256(b"precommitted-random-order-seed").hexdigest()
    protocol_id = "AV-RANK-CS-TEST-v1"
    order_commitment = make_order_commitment(
        protocol_id, episode_ids, episode_order, seed_commitment
    )
    if scores is None:
        scores = {arm: (index + 1) / (len(arms) + 1) for index, arm in enumerate(arms)}
    items = [
        {"episode_id": episode_id, "scores": dict(scores)}
        for episode_id in episode_order
    ]
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": 1,
        "protocol_id": protocol_id,
        "metric_id": "decision_accuracy",
        "arms": list(arms),
        "alpha": alpha,
        "betting_fractions": list(lambdas),
        "episode_ids": episode_ids,
        "episode_order": episode_order,
        "seed_commitment": seed_commitment,
        "order_commitment_sha256": order_commitment,
        "items": items,
    }


class RankConfidenceSequenceTests(unittest.TestCase):
    def test_ordered_partition_counts(self):
        self.assertEqual(len(weak_orders(2)), 3)
        self.assertEqual(len(weak_orders(3)), 13)
        self.assertEqual(len(weak_orders(6)), 4683)

    def test_equal_arms_remain_tied_and_all_ranks_possible(self):
        doc = make_document(
            arms=("A0", "A1", "A2"),
            scores={"A0": 0.5, "A1": 0.5, "A2": 0.5},
        )
        result = evaluate_document(doc)
        last = result["looks"][-1]
        self.assertEqual(last["surviving_weak_order_count"], 13)
        self.assertEqual(last["certified_dominances"], [])
        for arm in doc["arms"]:
            self.assertEqual(last["rank_sets"][arm]["ranks"], [1, 2, 3])
        self.assertTrue(last["shortcut_is_subset_of_exact"])

    def test_strict_order_is_certified(self):
        doc = make_document(
            arms=("A0", "A1", "A2"),
            scores={"A0": 0.0, "A1": 0.5, "A2": 1.0},
        )
        result = evaluate_document(doc)
        last = result["looks"][-1]
        certified = {
            (entry["better"], entry["worse"])
            for entry in last["certified_dominances"]
        }
        self.assertIn(("A2", "A1"), certified)
        self.assertIn(("A2", "A0"), certified)
        self.assertIn(("A1", "A0"), certified)
        self.assertEqual(last["rank_sets"]["A2"]["ranks"], [1])
        self.assertEqual(last["rank_sets"]["A1"]["ranks"], [2])
        self.assertEqual(last["rank_sets"]["A0"]["ranks"], [3])

    def test_six_arm_equal_score_known_answer(self):
        arms = ("A0", "A1", "A2", "A3", "A4", "A5")
        doc = make_document(arms=arms, scores={arm: 0.25 for arm in arms})
        result = evaluate_document(doc)
        self.assertEqual(result["weak_order_count"], 4683)
        last = result["looks"][-1]
        self.assertEqual(last["surviving_weak_order_count"], 4683)
        self.assertEqual(last["certified_dominances"], [])
        for arm in arms:
            self.assertEqual(last["rank_sets"][arm]["ranks"], [1, 2, 3, 4, 5, 6])
        self.assertTrue(last["shortcut_is_subset_of_exact"])

    def test_shortcut_never_claims_more_than_exact_result(self):
        doc = make_document(
            arms=("A0", "A1", "A2", "A3"),
            scores={"A0": 0.1, "A1": 0.35, "A2": 0.65, "A3": 0.9},
        )
        result = evaluate_document(doc)
        for look in result["looks"]:
            self.assertTrue(look["shortcut_is_subset_of_exact"])
            exact = {
                (row["better"], row["worse"])
                for row in look["certified_dominances"]
            }
            shortcut = {
                (row["better"], row["worse"])
                for row in look["shortcut_dominances"]
            }
            self.assertTrue(shortcut.issubset(exact))

    def test_order_commitment_is_verified(self):
        doc = make_document()
        doc["order_commitment_sha256"] = "0" * 64
        with self.assertRaises(RankEvaluationError):
            evaluate_document(doc)

    def test_item_order_must_match_precommitted_order(self):
        doc = make_document()
        doc["items"][0], doc["items"][1] = doc["items"][1], doc["items"][0]
        with self.assertRaises(RankEvaluationError):
            evaluate_document(doc)

    def test_missing_arm_score_fails_closed(self):
        doc = make_document()
        del doc["items"][0]["scores"]["A1"]
        with self.assertRaises(RankEvaluationError):
            evaluate_document(doc)

    def test_unknown_score_arm_fails_closed(self):
        doc = make_document()
        doc["items"][0]["scores"]["evaluator_truth"] = 1.0
        with self.assertRaises(RankEvaluationError):
            evaluate_document(doc)

    def test_out_of_range_and_non_finite_scores_fail_closed(self):
        for value in (-0.01, 1.01, float("nan"), float("inf"), True):
            with self.subTest(value=repr(value)):
                doc = make_document()
                doc["items"][0]["scores"]["A0"] = value
                with self.assertRaises(RankEvaluationError):
                    evaluate_document(doc)

    def test_invalid_lambda_grid_fails_closed(self):
        for grid in ([], [0.0], [1.0], [0.2, 0.2], [float("nan")]):
            with self.subTest(grid=grid):
                doc = make_document()
                doc["betting_fractions"] = grid
                with self.assertRaises(RankEvaluationError):
                    evaluate_document(doc)

    def test_report_has_replayable_transcript_and_metric_identity(self):
        doc = make_document()
        result = evaluate_document(doc)
        self.assertEqual(result["metric_id"], "decision_accuracy")
        self.assertEqual(result["episode_count"], len(doc["episode_ids"]))
        self.assertEqual(len(result["score_transcript_sha256"]), 64)
        self.assertEqual(
            result["looks"][-1]["transcript_chain_sha256"],
            result["score_transcript_sha256"],
        )


if __name__ == "__main__":
    unittest.main()
