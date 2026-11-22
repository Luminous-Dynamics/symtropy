import hashlib
import math
import random
import unittest

from margin_qualified_pareto import (
    BENCHMARK_ID,
    BettingEProcess,
    ParetoEvaluationError,
    evaluate_document,
    make_order_commitment,
)


def make_document(
    *,
    policies=("A", "B"),
    axes=None,
    scores_by_policy=None,
    hard_gates=None,
    episode_count=40,
    alpha=0.05,
):
    if axes is None:
        axes = [
            {
                "metric_id": "task_quality",
                "unit": "normalized score",
                "source_direction": "maximize",
                "score_direction": "higher_is_better",
                "transform_id": "quality-identity-v1",
                "provenance": "SYNTHETIC",
                "no_worse_tolerance": 0.05,
                "material_improvement_margin": 0.05,
            },
            {
                "metric_id": "energy_efficiency",
                "unit": "normalized utility",
                "source_direction": "minimize",
                "score_direction": "higher_is_better",
                "transform_id": "energy-benefit-1-minus-ratio-v1",
                "provenance": "SYNTHETIC",
                "no_worse_tolerance": 0.05,
                "material_improvement_margin": 0.05,
            },
        ]
    if hard_gates is None:
        hard_gates = {
            policy: {"eligible": True, "failed_constraints": []}
            for policy in policies
        }
    eligible = [policy for policy in policies if hard_gates[policy]["eligible"]]
    if scores_by_policy is None:
        scores_by_policy = {
            "A": {"task_quality": 0.9, "energy_efficiency": 0.9},
            "B": {"task_quality": 0.2, "energy_efficiency": 0.2},
        }
    episode_ids = [f"episode-{index:04d}" for index in range(episode_count)]
    episode_order = list(reversed(episode_ids))
    seed_commitment = hashlib.sha256(b"pareto-cs-precommitted-seed").hexdigest()
    protocol_id = "AV-PARETO-CS-TEST-v1"
    order_commitment = make_order_commitment(
        protocol_id, episode_ids, episode_order, seed_commitment
    )
    items = [
        {
            "episode_id": episode_id,
            "scores": {
                policy: dict(scores_by_policy[policy])
                for policy in eligible
            },
        }
        for episode_id in episode_order
    ]
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": 1,
        "protocol_id": protocol_id,
        "policies": list(policies),
        "axes": axes,
        "alpha": alpha,
        "betting_fractions": [0.03, 0.06, 0.12, 0.25, 0.5],
        "episode_ids": episode_ids,
        "episode_order": episode_order,
        "seed_commitment": seed_commitment,
        "order_commitment_sha256": order_commitment,
        "hard_gates": hard_gates,
        "items": items,
    }


class MarginQualifiedParetoTests(unittest.TestCase):
    def test_identical_policies_do_not_create_material_dominance(self):
        doc = make_document(
            scores_by_policy={
                "A": {"task_quality": 0.5, "energy_efficiency": 0.5},
                "B": {"task_quality": 0.5, "energy_efficiency": 0.5},
            }
        )
        result = evaluate_document(doc)
        last = result["looks"][-1]
        self.assertEqual(last["pair_certificates"]["A->B"]["status"], "UNRESOLVED")
        self.assertEqual(last["pair_certificates"]["B->A"]["status"], "UNRESOLVED")
        self.assertEqual(last["certified_dominance_edges"], [])
        self.assertTrue(all(x["status"] == "POSSIBLY_NONDOMINATED" for x in last["inferential_frontier"].values()))

    def test_strict_improvement_on_all_axes_certifies_directed_edge(self):
        doc = make_document(
            scores_by_policy={
                "A": {"task_quality": 0.95, "energy_efficiency": 0.95},
                "B": {"task_quality": 0.15, "energy_efficiency": 0.15},
            }
        )
        result = evaluate_document(doc)
        last = result["looks"][-1]
        self.assertEqual(last["pair_certificates"]["A->B"]["status"], "CERTIFIED_DOMINATES")
        self.assertIn({"better": "A", "worse": "B"}, last["certified_dominance_edges"])
        self.assertEqual(last["inferential_frontier"]["B"]["status"], "CERTIFIED_DOMINATED")
        self.assertEqual(last["inferential_frontier"]["A"]["status"], "POSSIBLY_NONDOMINATED")

    def test_quality_energy_tradeoff_does_not_force_a_total_order(self):
        doc = make_document(
            scores_by_policy={
                "A": {"task_quality": 0.95, "energy_efficiency": 0.15},
                "B": {"task_quality": 0.15, "energy_efficiency": 0.95},
            }
        )
        last = evaluate_document(doc)["looks"][-1]
        self.assertEqual(last["certified_dominance_edges"], [])
        self.assertEqual(last["pair_certificates"]["A->B"]["status"], "UNRESOLVED")
        self.assertEqual(last["pair_certificates"]["B->A"]["status"], "UNRESOLVED")

    def test_one_axis_beyond_no_worse_margin_blocks_dominance(self):
        doc = make_document(
            scores_by_policy={
                "A": {"task_quality": 0.99, "energy_efficiency": 0.1},
                "B": {"task_quality": 0.1, "energy_efficiency": 0.9},
            }
        )
        last = evaluate_document(doc)["looks"][-1]
        self.assertFalse(last["pair_certificates"]["A->B"]["all_axes_noninferior"])
        self.assertNotEqual(last["pair_certificates"]["A->B"]["status"], "CERTIFIED_DOMINATES")

    def test_hard_gate_ineligible_policy_is_not_scored_or_rescued(self):
        gates = {
            "A": {"eligible": True, "failed_constraints": []},
            "B": {"eligible": False, "failed_constraints": ["energy_cap_exceeded"]},
        }
        doc = make_document(
            hard_gates=gates,
            scores_by_policy={"A": {"task_quality": 0.9, "energy_efficiency": 0.9}},
        )
        last = evaluate_document(doc)["looks"][-1]
        self.assertEqual(last["pair_certificates"]["A->B"]["status"], "INELIGIBLE")
        self.assertEqual(last["pair_certificates"]["B->A"]["status"], "INELIGIBLE")
        self.assertEqual(last["inferential_frontier"]["B"]["status"], "INELIGIBLE")
        self.assertEqual(last["inferential_frontier"]["A"]["status"], "POSSIBLY_NONDOMINATED")

    def test_directional_eprocess_first_look_matches_independent_formula(self):
        process = BettingEProcess(
            cohort_size=10,
            alpha=0.01,
            betting_fractions=[0.1, 0.2, 0.3],
        )
        result = process.update(1.0)
        mean_lambda = (0.1 + 0.2 + 0.3) / 3
        self.assertAlmostEqual(result["log_wealth"], math.log(1.0 + mean_lambda), places=12)
        self.assertEqual(result["offset"], 0.0)

    def test_alpha_is_allocated_across_directions_pairs_and_axes(self):
        doc = make_document()
        result = evaluate_document(doc)
        self.assertEqual(result["directional_test_count"], 2 * 2 * 2 * 1)
        self.assertAlmostEqual(result["alpha_local"], 0.05 / 8)

    def test_measured_axis_requires_method_and_evidence_digest(self):
        axes = [
            {
                "metric_id": "latency",
                "unit": "milliseconds",
                "source_direction": "minimize",
                "score_direction": "higher_is_better",
                "transform_id": "latency-benefit-v1",
                "provenance": "MEASURED",
                "no_worse_tolerance": 0.05,
                "material_improvement_margin": 0.05,
            }
        ]
        doc = make_document(axes=axes)
        with self.assertRaises(ParetoEvaluationError):
            evaluate_document(doc)
        axes[0]["measurement_method"] = "hardware-counter-v1"
        axes[0]["evidence_digest_sha256"] = hashlib.sha256(b"metered-evidence").hexdigest()
        doc = make_document(
            axes=axes,
            scores_by_policy={"A": {"latency": 0.9}, "B": {"latency": 0.2}},
        )
        result = evaluate_document(doc)
        self.assertEqual(result["evidence_scope"], "MEASURED_AXES_ONLY")

    def test_random_correlated_metric_vectors_replay(self):
        rng = random.Random(6840)
        policies = ("A", "B", "C")
        axes = [
            {
                "metric_id": metric,
                "unit": "normalized utility",
                "source_direction": "maximize",
                "score_direction": "higher_is_better",
                "transform_id": f"{metric}-frozen-transform-v1",
                "provenance": "SYNTHETIC",
                "no_worse_tolerance": 0.05,
                "material_improvement_margin": 0.05,
            }
            for metric in ("quality", "silence", "latency_benefit")
        ]
        episode_count = 35
        ids = [f"episode-{index:04d}" for index in range(episode_count)]
        order = list(reversed(ids))
        seed = hashlib.sha256(b"random-correlated-score-order").hexdigest()
        protocol = "AV-PARETO-RANDOM-v1"
        commitment = make_order_commitment(protocol, ids, order, seed)
        items = []
        for episode_id in order:
            common = rng.random()
            item_scores = {}
            for policy in policies:
                item_scores[policy] = {
                    metric: 0.65 * common + 0.35 * rng.random()
                    for metric in ("quality", "silence", "latency_benefit")
                }
            items.append({"episode_id": episode_id, "scores": item_scores})
        doc = {
            "benchmark_id": BENCHMARK_ID,
            "schema_version": 1,
            "protocol_id": protocol,
            "policies": list(policies),
            "axes": axes,
            "alpha": 0.05,
            "betting_fractions": [0.03, 0.06, 0.12, 0.25, 0.5],
            "episode_ids": ids,
            "episode_order": order,
            "seed_commitment": seed,
            "order_commitment_sha256": commitment,
            "hard_gates": {
                policy: {"eligible": True, "failed_constraints": []}
                for policy in policies
            },
            "items": items,
        }
        result = evaluate_document(doc)
        self.assertEqual(len(result["looks"]), episode_count)
        self.assertEqual(result["directional_test_count"], 2 * 3 * 3 * 2)
        for look in result["looks"]:
            for pair_report in look["pair_certificates"].values():
                if pair_report["status"] != "INELIGIBLE":
                    self.assertIn(pair_report["status"], {"UNRESOLVED", "CERTIFIED_DOMINATES"})

    def test_order_commitment_and_row_order_are_enforced(self):
        doc = make_document()
        doc["order_commitment_sha256"] = "0" * 64
        with self.assertRaises(ParetoEvaluationError):
            evaluate_document(doc)
        doc = make_document()
        doc["items"][0], doc["items"][1] = doc["items"][1], doc["items"][0]
        with self.assertRaises(ParetoEvaluationError):
            evaluate_document(doc)

    def test_incomplete_axis_matrix_fails_closed(self):
        doc = make_document()
        del doc["items"][0]["scores"]["A"]["energy_efficiency"]
        with self.assertRaises(ParetoEvaluationError):
            evaluate_document(doc)


if __name__ == "__main__":
    unittest.main()
