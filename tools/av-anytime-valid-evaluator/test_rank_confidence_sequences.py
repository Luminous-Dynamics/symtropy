import hashlib
import math
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


def make_retirement_document(
    *,
    rule="TOP_K_RESOLVED",
    top_k=3,
    episode_count=120,
    scores=None,
):
    arms = ("A0", "A1", "A2", "A3", "A4", "A5")
    if scores is None:
        scores = {
            "A0": 0.05,
            "A1": 0.20,
            "A2": 0.35,
            "A3": 0.65,
            "A4": 0.80,
            "A5": 0.95,
        }
    episode_ids = [f"episode-{index:04d}" for index in range(episode_count)]
    episode_order = list(reversed(episode_ids))
    seed_commitment = hashlib.sha256(b"retirement-fixed-random-order-seed").hexdigest()
    protocol_id = "AV-RANK-RETIREMENT-TEST-v1"
    document = {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": 1,
        "protocol_id": protocol_id,
        "metric_id": "decision_accuracy",
        "arms": list(arms),
        "alpha": 0.05,
        "betting_fractions": [0.03, 0.06, 0.12, 0.25, 0.5],
        "episode_ids": episode_ids,
        "episode_order": episode_order,
        "seed_commitment": seed_commitment,
        "order_commitment_sha256": make_order_commitment(
            protocol_id, episode_ids, episode_order, seed_commitment
        ),
        "retirement_rule": rule,
        "items": [],
    }
    if rule == "TOP_K_RESOLVED":
        document["retirement_top_k"] = top_k

    active = list(arms)
    for episode_id in episode_order:
        item = {
            "episode_id": episode_id,
            "active_arms": list(active),
            "scores": {arm: scores[arm] for arm in active},
        }
        document["items"].append(item)
        partial = evaluate_document(document)
        active = partial["active_arms"]
        if not active:
            break
    return document


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

    def test_first_look_pairwise_wealth_matches_independent_formula(self):
        doc = make_document(
            arms=("A0", "A1"),
            scores={"A0": 0.0, "A1": 1.0},
        )
        result = evaluate_document(doc)
        first = result["looks"][0]["pair_log_wealths"]
        mean_lambda = sum((0.03, 0.06, 0.12, 0.25, 0.5)) / 5
        self.assertAlmostEqual(first["A1>A0"], math.log(1.0 + mean_lambda), places=12)
        self.assertAlmostEqual(first["A0>A1"], math.log(1.0 - mean_lambda), places=12)

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
        self.assertEqual(last["tiers"]["by_arm"], {"A0": 3, "A1": 2, "A2": 1})
        self.assertEqual(last["rank_bounds_from_dominances"]["A2"], {"lower": 1, "upper": 1})

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
        self.assertEqual(set(last["tiers"]["groups"]), {"1"})
        self.assertTrue(all(tier == 1 for tier in last["tiers"]["by_arm"].values()))

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

    def test_random_score_vectors_preserve_shortcut_subset_invariant(self):
        import random

        rng = random.Random(20261122)
        arms = ("A0", "A1", "A2", "A3", "A4", "A5")
        episode_ids = [f"episode-{index:04d}" for index in range(40)]
        episode_order = list(reversed(episode_ids))
        seed_commitment = hashlib.sha256(b"randomized-fixed-order-seed").hexdigest()
        protocol_id = "AV-RANK-CS-RANDOM-v1"
        order_commitment = make_order_commitment(
            protocol_id, episode_ids, episode_order, seed_commitment
        )
        items = []
        for episode_id in episode_order:
            shared = rng.random()
            scores = {
                arm: min(1.0, max(0.0, 0.7 * shared + 0.3 * rng.random()))
                for arm in arms
            }
            items.append({"episode_id": episode_id, "scores": scores})
        doc = {
            "benchmark_id": BENCHMARK_ID,
            "schema_version": 1,
            "protocol_id": protocol_id,
            "metric_id": "decision_accuracy",
            "arms": list(arms),
            "alpha": 0.05,
            "betting_fractions": [0.03, 0.06, 0.12, 0.25, 0.5],
            "episode_ids": episode_ids,
            "episode_order": episode_order,
            "seed_commitment": seed_commitment,
            "order_commitment_sha256": order_commitment,
            "items": items,
        }
        result = evaluate_document(doc)
        self.assertEqual(len(result["looks"]), len(episode_ids))
        for look in result["looks"]:
            self.assertTrue(look["shortcut_is_subset_of_exact"])
            for arm in arms:
                self.assertTrue(look["rank_sets"][arm]["ranks"])
                self.assertLessEqual(
                    look["rank_bounds_from_dominances"][arm]["lower"],
                    look["rank_bounds_from_dominances"][arm]["upper"],
                )

    def test_exact_enumerator_bounds_model_count(self):
        doc = make_document(arms=tuple(f"A{i}" for i in range(9)))
        with self.assertRaises(RankEvaluationError):
            evaluate_document(doc)

    def test_order_commitment_is_verified(self):
        doc = make_document()
        doc["order_commitment_sha256"] = "0" * 64
        with self.assertRaises(RankEvaluationError):
            evaluate_document(doc)

        doc = make_document()
        doc["episode_order"][0] = []
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

    def test_no_retirement_reproduces_full_all_arm_execution(self):
        doc = make_document()
        result = evaluate_document(doc)
        self.assertEqual(result["completion_status"], "COHORT_EXHAUSTED")
        self.assertEqual(result["episodes_processed"], len(doc["episode_ids"]))
        self.assertEqual(
            result["model_invocation_counts"],
            {arm: len(doc["episode_ids"]) for arm in doc["arms"]},
        )
        self.assertEqual(result["model_invocations_avoided_vs_full"], {arm: 0 for arm in doc["arms"]})
        self.assertEqual(result["retirement_rule"], "NONE")

    def test_top_k_retirement_stops_model_scoring_early(self):
        doc = make_retirement_document(rule="TOP_K_RESOLVED", top_k=3)
        result = evaluate_document(doc)
        self.assertEqual(result["completion_status"], "ALL_MODELS_RETIRED")
        self.assertLess(result["episodes_processed"], result["episode_count"])
        self.assertEqual(result["active_arms"], [])
        self.assertTrue(result["retirement_log"])
        self.assertTrue(any(value > 0 for value in result["model_invocations_avoided_vs_full"].values()))
        for look in result["looks"]:
            self.assertEqual(
                set(look["score_vector"]),
                set(look["active_arms"]),
            )
            self.assertEqual(
                look["next_active_arms"],
                [arm for arm in look["active_arms"] if arm not in look["retired_this_look"]],
            )

    def test_pairwise_wealth_freezes_after_either_model_retires(self):
        doc = make_retirement_document(rule="TOP_K_RESOLVED", top_k=3)
        result = evaluate_document(doc)
        retirement_look = next(
            (index for index, look in enumerate(result["looks"]) if look["retired_this_look"]),
            None,
        )
        self.assertIsNotNone(retirement_look)
        look = result["looks"][retirement_look]
        if retirement_look + 1 < len(result["looks"]):
            next_look = result["looks"][retirement_look + 1]
            for retired_arm in look["retired_this_look"]:
                for other in result["arms"]:
                    if other == retired_arm:
                        continue
                    key_one = f"{retired_arm}>{other}"
                    key_two = f"{other}>{retired_arm}"
                    self.assertAlmostEqual(
                        look["pair_log_wealths"][key_one],
                        next_look["pair_log_wealths"][key_one],
                        places=12,
                    )
                    self.assertAlmostEqual(
                        look["pair_log_wealths"][key_two],
                        next_look["pair_log_wealths"][key_two],
                        places=12,
                    )

    def test_retired_models_cannot_be_reintroduced(self):
        doc = make_retirement_document(rule="TOP_K_RESOLVED", top_k=3)
        result = evaluate_document(doc)
        retirement_look = next(
            (index for index, look in enumerate(result["looks"]) if look["retired_this_look"]),
            None,
        )
        self.assertIsNotNone(retirement_look)
        if retirement_look + 1 >= len(doc["items"]):
            self.skipTest("all models retired on final supplied item")
        retired = result["looks"][retirement_look]["retired_this_look"][0]
        doc["items"][retirement_look + 1]["active_arms"].append(retired)
        doc["items"][retirement_look + 1]["scores"][retired] = 0.5
        with self.assertRaises(RankEvaluationError):
            evaluate_document(doc)

    def test_score_vector_must_match_active_set_exactly(self):
        doc = make_retirement_document(rule="TOP_K_RESOLVED", top_k=3)
        result = evaluate_document(doc)
        if result["episodes_processed"] < 2:
            self.skipTest("only one item was evaluated")
        doc["items"][1]["scores"].pop(doc["items"][1]["active_arms"][0])
        with self.assertRaises(RankEvaluationError):
            evaluate_document(doc)

    def test_tied_models_are_not_retired_on_point_estimates(self):
        arms = ("A0", "A1", "A2", "A3", "A4", "A5")
        equal_scores = {arm: 0.5 for arm in arms}
        doc = make_retirement_document(
            rule="TOP_K_RESOLVED",
            top_k=3,
            episode_count=24,
            scores=equal_scores,
        )
        result = evaluate_document(doc)
        self.assertEqual(result["completion_status"], "COHORT_EXHAUSTED")
        self.assertEqual(result["retirement_log"], {})
        self.assertEqual(result["model_invocation_counts"], {arm: 24 for arm in arms})

    def test_singleton_rank_retirement_uses_exact_rank_sets(self):
        doc = make_retirement_document(rule="RANK_SINGLETON")
        result = evaluate_document(doc)
        for look in result["looks"]:
            for retirement in look["retirement_decisions"]:
                self.assertEqual(retirement["rule"], "RANK_SINGLETON")
                self.assertEqual(
                    len(look["rank_sets"][retirement["arm"]]["ranks"]),
                    1,
                )

    def test_pairwise_resolved_retirement_requires_all_comparisons(self):
        doc = make_retirement_document(rule="PAIRWISE_RESOLVED")
        result = evaluate_document(doc)
        for look in result["looks"]:
            certified = {
                (row["better"], row["worse"])
                for row in look["certified_dominances"]
            }
            for retirement in look["retirement_decisions"]:
                arm = retirement["arm"]
                for other in doc["arms"]:
                    if other != arm:
                        self.assertTrue((arm, other) in certified or (other, arm) in certified)

    def test_invalid_retirement_policy_and_top_k_fail_closed(self):
        doc = make_retirement_document(rule="TOP_K_RESOLVED", top_k=3)
        doc["retirement_rule"] = []
        with self.assertRaises(RankEvaluationError):
            evaluate_document(doc)
        doc = make_retirement_document(rule="TOP_K_RESOLVED", top_k=3)
        doc["retirement_top_k"] = True
        with self.assertRaises(RankEvaluationError):
            evaluate_document(doc)

    def test_report_has_replayable_transcript_and_metric_identity(self):
        doc = make_document()
        result = evaluate_document(doc)
        self.assertEqual(result["metric_id"], "decision_accuracy")
        self.assertEqual(result["episode_count"], len(doc["episode_ids"]))
        self.assertEqual(len(result["score_transcript_sha256"]), 64)
        self.assertEqual(len(result["looks"][-1]["pair_log_wealths"]), len(doc["arms"]) * (len(doc["arms"]) - 1))
        self.assertEqual(
            result["looks"][-1]["transcript_chain_sha256"],
            result["score_transcript_sha256"],
        )


if __name__ == "__main__":
    unittest.main()
