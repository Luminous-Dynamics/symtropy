#!/usr/bin/env python3
"""Finite-cohort rank confidence sequences for matched model score vectors.

Reference implementation of Khosravi & Huo (2026), Rank Confidence
Sequences: Anytime-valid Leaderboards, for the finite-benchmark sampling
model. Models are scored on the same fixed episodes, in a custodian-committed
order generated independently of their results.

The evaluator handles one bounded [0, 1], higher-is-better scalar metric.
It does not cover data-adaptive outer episode selection, multiple metrics,
unbounded costs, or Pareto certification.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from typing import Any


BENCHMARK_ID = "av-multiarm-rank-cs-v0"
METHOD_ID = "khosravi-huo-rank-confidence-sequences-2026-finite-v0"
DEFAULT_LAMBDAS = (0.03, 0.06, 0.12, 0.25, 0.5)


class RankEvaluationError(ValueError):
    """Malformed or unsupported RCS input."""


def _finite_number(value: Any) -> bool:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return False
    try:
        return math.isfinite(float(value))
    except (OverflowError, TypeError, ValueError):
        return False


def _logsumexp(values: list[float]) -> float:
    if not values:
        raise RankEvaluationError("logsumexp requires at least one value")
    maximum = max(values)
    if maximum == -math.inf:
        return -math.inf
    return maximum + math.log(sum(math.exp(value - maximum) for value in values))


def _canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def _sha256_hex(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def make_order_commitment(
    protocol_id: str,
    episode_ids: list[str],
    episode_order: list[str],
    seed_commitment: str,
) -> str:
    """Digest the precommitted finite-cohort order envelope."""
    return _sha256_hex(_canonical_json({
        "protocol_id": protocol_id,
        "episode_ids": episode_ids,
        "episode_order": episode_order,
        "seed_commitment": seed_commitment,
    }))


def weak_orders(model_count: int) -> list[tuple[tuple[int, ...], ...]]:
    """Enumerate ordered set partitions, with groups ordered worst to best."""
    if isinstance(model_count, bool) or not isinstance(model_count, int) or model_count < 1:
        raise RankEvaluationError("model_count must be a positive integer")
    orders: list[tuple[tuple[int, ...], ...]] = [tuple()]
    for model in range(model_count):
        extended: list[tuple[tuple[int, ...], ...]] = []
        for order in orders:
            for group_index in range(len(order)):
                groups = list(order)
                groups[group_index] = groups[group_index] + (model,)
                extended.append(tuple(groups))
            for position in range(len(order) + 1):
                groups = list(order)
                groups.insert(position, (model,))
                extended.append(tuple(groups))
        orders = extended
    return orders


def _weak_order_pair_indices(
    order: tuple[tuple[int, ...], ...],
    pair_index: dict[tuple[int, int], int],
) -> tuple[int, ...]:
    levels: dict[int, int] = {}
    for level, group in enumerate(order):
        for model in group:
            levels[model] = level
    pairs = []
    for (left, right), index in pair_index.items():
        # left <=_W right means right is at least as good as left.
        if levels[left] <= levels[right]:
            pairs.append(index)
    return tuple(pairs)


def _rank_report(
    surviving: list[tuple[tuple[int, ...], ...]],
    arms: list[str],
    pair_index: dict[tuple[int, int], int],
    pair_log_wealths: list[float],
    alpha: float,
) -> dict[str, Any]:
    model_count = len(arms)
    rank_sets: dict[str, set[int]] = {arm: set() for arm in arms}
    levels_by_order: list[dict[int, int]] = []

    for order in surviving:
        levels: dict[int, int] = {}
        for level, group in enumerate(order):
            for model in group:
                levels[model] = level
        levels_by_order.append(levels)
        for model, arm in enumerate(arms):
            rank = 1 + sum(levels[other] > levels[model] for other in range(model_count))
            rank_sets[arm].add(rank)

    certified = []
    unresolved = []
    shortcut_certified = []
    for left in range(model_count):
        for right in range(model_count):
            if left == right:
                continue

            # Published polynomial shortcut, evaluated for every ordered pair.
            terms = [pair_log_wealths[pair_index[(left, right)]]]
            for middle in range(model_count):
                if middle in (left, right):
                    continue
                terms.append(min(
                    pair_log_wealths[pair_index[(left, middle)]],
                    pair_log_wealths[pair_index[(middle, right)]],
                ))
            log_b = _logsumexp(terms)
            shortcut_threshold = math.log(model_count * (model_count - 1) / alpha)
            if log_b >= shortcut_threshold:
                shortcut_certified.append({
                    "better": arms[left],
                    "worse": arms[right],
                })

            left_better = all(
                levels[left] > levels[right] for levels in levels_by_order
            )
            if left_better:
                certified.append({"better": arms[left], "worse": arms[right]})
                continue
            right_better = all(
                levels[right] > levels[left] for levels in levels_by_order
            )
            if not right_better and left < right:
                unresolved.append({"pair": [arms[left], arms[right]]})

    rendered_rank_sets = {}
    for arm, values in rank_sets.items():
        ordered = sorted(values)
        rendered_rank_sets[arm] = {
            "ranks": ordered,
            "lower": ordered[0],
            "upper": ordered[-1],
        }

    top_k = {}
    for k in range(1, model_count + 1):
        inside = []
        outside = []
        unresolved_membership = []
        for arm, values in rank_sets.items():
            if max(values) <= k:
                inside.append(arm)
            elif min(values) > k:
                outside.append(arm)
            else:
                unresolved_membership.append(arm)
        top_k[str(k)] = {
            "certified_inside": inside,
            "certified_outside": outside,
            "unresolved": unresolved_membership,
        }

    exact_set = {
        (item["better"], item["worse"]) for item in certified
    }
    arm_index = {arm: index for index, arm in enumerate(arms)}
    exact_index_set = {
        (arm_index[better], arm_index[worse])
        for better, worse in exact_set
    }
    tier_cache: dict[int, int] = {}
    tier_visiting: set[int] = set()

    def tier_for(model: int) -> int:
        if model in tier_cache:
            return tier_cache[model]
        if model in tier_visiting:
            raise RankEvaluationError("certified dominance graph contains a cycle")
        tier_visiting.add(model)
        better_models = [
            left for left, right in exact_index_set if right == model
        ]
        tier = 1 + max(
            (tier_for(better) for better in better_models),
            default=0,
        )
        tier_visiting.remove(model)
        tier_cache[model] = tier
        return tier

    tiers_by_model = {
        arm: tier_for(model_index) for model_index, arm in enumerate(arms)
    }
    tier_groups: dict[int, list[str]] = {}
    for arm, tier in tiers_by_model.items():
        tier_groups.setdefault(tier, []).append(arm)

    rank_bounds = {}
    for model, arm in enumerate(arms):
        better_count = sum((left, model) in exact_index_set for left in range(model_count))
        worse_count = sum((model, right) in exact_index_set for right in range(model_count))
        rank_bounds[arm] = {
            "lower": 1 + better_count,
            "upper": model_count - worse_count,
        }

    shortcut_set = {
        (item["better"], item["worse"]) for item in shortcut_certified
    }
    return {
        "surviving_weak_order_count": len(surviving),
        "certified_dominances": certified,
        "unresolved_pairs": unresolved,
        "rank_sets": rendered_rank_sets,
        "rank_bounds_from_dominances": rank_bounds,
        "top_k": top_k,
        "tiers": {
            "by_arm": tiers_by_model,
            "groups": {
                str(tier): group for tier, group in sorted(tier_groups.items())
            },
        },
        "shortcut_dominances": shortcut_certified,
        "shortcut_is_subset_of_exact": shortcut_set.issubset(exact_set),
    }


def _select_retirements(
    report: dict[str, Any],
    active_arms: list[str],
    rule: str,
    top_k_target: int | None,
) -> list[dict[str, Any]]:
    """Choose predictable retirements using only the just-observed rank report."""
    if rule == "NONE":
        return []
    certified = {
        (row["better"], row["worse"])
        for row in report["certified_dominances"]
    }
    retirements = []
    for arm in active_arms:
        evidence: dict[str, Any] | None = None
        if rule == "TOP_K_RESOLVED":
            membership = report["top_k"][str(top_k_target)]
            if arm in membership["certified_inside"]:
                evidence = {"status": "CERTIFIED_INSIDE", "k": top_k_target}
            elif arm in membership["certified_outside"]:
                evidence = {"status": "CERTIFIED_OUTSIDE", "k": top_k_target}
        elif rule == "RANK_SINGLETON":
            ranks = report["rank_sets"][arm]["ranks"]
            if len(ranks) == 1:
                evidence = {"status": "SINGLETON_RANK", "rank": ranks[0]}
        elif rule == "PAIRWISE_RESOLVED":
            unresolved = [
                other for other in report["rank_sets"]
                if other != arm
                and (arm, other) not in certified
                and (other, arm) not in certified
            ]
            if not unresolved:
                evidence = {"status": "ALL_PAIRWISE_COMPARISONS_RESOLVED"}
        if evidence is not None:
            retirements.append({"arm": arm, "rule": rule, "evidence": evidence})
    return retirements


def evaluate_document(document: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(document, dict):
        raise RankEvaluationError("input root must be an object")
    if document.get("benchmark_id") != BENCHMARK_ID:
        raise RankEvaluationError("benchmark_id mismatch")
    version = document.get("schema_version")
    if isinstance(version, bool) or not isinstance(version, int) or version != 1:
        raise RankEvaluationError("schema_version must be integer 1")
    protocol_id = document.get("protocol_id")
    metric_id = document.get("metric_id")
    if not isinstance(protocol_id, str) or not protocol_id:
        raise RankEvaluationError("protocol_id is required")
    if not isinstance(metric_id, str) or not metric_id:
        raise RankEvaluationError("metric_id is required")

    arms = document.get("arms")
    if not isinstance(arms, list) or len(arms) < 2:
        raise RankEvaluationError("arms must contain at least two model IDs")
    if len(arms) > 8:
        raise RankEvaluationError("exact weak-order reference supports at most eight arms")
    if any(not isinstance(arm, str) or not arm for arm in arms):
        raise RankEvaluationError("arm IDs must be non-empty strings")
    if len(set(arms)) != len(arms):
        raise RankEvaluationError("arm IDs must be unique")

    alpha = document.get("alpha", 0.05)
    if not _finite_number(alpha) or not 0.0 < alpha < 1.0:
        raise RankEvaluationError("alpha must be finite and strictly between 0 and 1")
    alpha = float(alpha)

    lambdas = document.get("betting_fractions", list(DEFAULT_LAMBDAS))
    if not isinstance(lambdas, list) or not lambdas:
        raise RankEvaluationError("betting_fractions must be a non-empty array")
    if any(not _finite_number(value) or not 0.0 < value < 1.0 for value in lambdas):
        raise RankEvaluationError("every betting fraction must be finite and in (0, 1)")
    lambdas = [float(value) for value in lambdas]
    if len(set(lambdas)) != len(lambdas):
        raise RankEvaluationError("betting_fractions must be unique")

    episode_ids = document.get("episode_ids")
    episode_order = document.get("episode_order")
    seed_commitment = document.get("seed_commitment")
    order_commitment = document.get("order_commitment_sha256")
    if not isinstance(episode_ids, list) or not episode_ids:
        raise RankEvaluationError("episode_ids must be a non-empty array")
    if any(not isinstance(item, str) or not item for item in episode_ids):
        raise RankEvaluationError("episode IDs must be non-empty strings")
    if len(set(episode_ids)) != len(episode_ids):
        raise RankEvaluationError("episode_ids must be unique")
    if episode_ids != sorted(episode_ids):
        raise RankEvaluationError("episode_ids must be sorted canonically")
    if not isinstance(episode_order, list) or len(episode_order) != len(episode_ids):
        raise RankEvaluationError("episode_order must contain every episode exactly once")
    if any(not isinstance(item, str) or not item for item in episode_order):
        raise RankEvaluationError("episode_order entries must be non-empty strings")
    if len(set(episode_order)) != len(episode_order):
        raise RankEvaluationError("episode_order must not contain duplicate IDs")
    if set(episode_order) != set(episode_ids):
        raise RankEvaluationError("episode_order must be a permutation of episode_ids")
    if not isinstance(seed_commitment, str) or len(seed_commitment) != 64:
        raise RankEvaluationError("seed_commitment must be a 64-character SHA-256 commitment")
    try:
        int(seed_commitment, 16)
    except ValueError as exc:
        raise RankEvaluationError("seed_commitment must be hexadecimal") from exc
    if not isinstance(order_commitment, str) or len(order_commitment) != 64:
        raise RankEvaluationError("order_commitment_sha256 must be a SHA-256 digest")
    expected_commitment = make_order_commitment(
        protocol_id, episode_ids, episode_order, seed_commitment
    )
    if order_commitment != expected_commitment:
        raise RankEvaluationError("precommitted order digest mismatch")

    retirement_rule = document.get("retirement_rule", "NONE")
    if retirement_rule not in {"NONE", "TOP_K_RESOLVED", "RANK_SINGLETON", "PAIRWISE_RESOLVED"}:
        raise RankEvaluationError("unsupported retirement_rule")
    top_k_target = document.get("retirement_top_k")
    if retirement_rule == "TOP_K_RESOLVED":
        if isinstance(top_k_target, bool) or not isinstance(top_k_target, int) or not 1 <= top_k_target <= len(arms):
            raise RankEvaluationError("TOP_K_RESOLVED requires retirement_top_k in [1, model_count]")
    elif top_k_target is not None:
        raise RankEvaluationError("retirement_top_k is only valid with TOP_K_RESOLVED")

    items = document.get("items")
    if not isinstance(items, list) or not items:
        raise RankEvaluationError("items must be a non-empty evaluated prefix")
    if len(items) > len(episode_order):
        raise RankEvaluationError("items cannot exceed the fixed episode cohort")
    if retirement_rule == "NONE" and len(items) != len(episode_order):
        raise RankEvaluationError("retirement_rule=NONE requires the full episode cohort")

    model_count = len(arms)
    pairs = [(left, right) for left in range(model_count) for right in range(model_count) if left != right]
    pair_index = {pair: index for index, pair in enumerate(pairs)}
    orders = weak_orders(model_count)
    order_pair_indices = [
        _weak_order_pair_indices(order, pair_index) for order in orders
    ]
    log_gamblers = [
        [[0.0 for _ in lambdas] for _ in pairs]
    ][0]
    cumulative_differences = [0.0 for _ in pairs]
    max_log_order_wealth = [-math.inf for _ in orders]
    alive = [True for _ in orders]
    threshold = -math.log(alpha)

    transcript_chain = hashlib.sha256(_canonical_json({
        "benchmark_id": BENCHMARK_ID,
        "protocol_id": protocol_id,
        "metric_id": metric_id,
        "arms": arms,
        "alpha": alpha,
        "betting_fractions": lambdas,
        "episode_ids": episode_ids,
        "episode_order": episode_order,
        "seed_commitment": seed_commitment,
        "order_commitment_sha256": order_commitment,
    })).digest()

    looks = []
    total_episodes = len(episode_order)
    expected_active_arms = list(arms)
    invocation_counts = {arm: 0 for arm in arms}
    retirement_log: dict[str, dict[str, Any]] = {}
    completion_status = "INCOMPLETE_INPUT"
    for t, item in enumerate(items, start=1):
        if not expected_active_arms:
            raise RankEvaluationError("no score rows may follow the all-models-retired decision")
        if not isinstance(item, dict) or item.get("episode_id") != episode_order[t - 1]:
            raise RankEvaluationError("score rows must follow the precommitted episode order")
        allowed_item_keys = {"episode_id", "scores"}
        if retirement_rule != "NONE":
            allowed_item_keys.add("active_arms")
        if set(item) != allowed_item_keys:
            raise RankEvaluationError(f"item {t}: unexpected or missing item fields")
        active_arms = item.get("active_arms", list(arms))
        if not isinstance(active_arms, list) or active_arms != expected_active_arms:
            raise RankEvaluationError("active_arms must exactly match the evaluator-issued prior active set")
        if len(set(active_arms)) != len(active_arms):
            raise RankEvaluationError("active_arms cannot contain duplicate models")
        scores = item.get("scores")
        if not isinstance(scores, dict) or set(scores) != set(active_arms):
            raise RankEvaluationError("each item must score every active arm and no retired/inactive arm")
        for arm in active_arms:
            value = scores[arm]
            if not _finite_number(value) or not 0.0 <= float(value) <= 1.0:
                raise RankEvaluationError(f"item {t}, {arm}: score must be finite and in [0,1]")
            invocation_counts[arm] += 1

        active_set = set(active_arms)
        for pair_no, (left, right) in enumerate(pairs):
            if arms[left] not in active_set or arms[right] not in active_set:
                continue
            z = float(scores[arms[left]]) - float(scores[arms[right]])
            remaining_including_current = total_episodes - t + 1
            offset = max(
                -0.99,
                min(1.0, -cumulative_differences[pair_no] / remaining_including_current),
            )
            denominator = 1.0 + offset
            for lambda_no, betting_fraction in enumerate(lambdas):
                factor = 1.0 + betting_fraction * (z - offset) / denominator
                if not math.isfinite(factor) or factor <= 0.0:
                    raise RankEvaluationError("pairwise betting factor is non-positive or non-finite")
                log_gamblers[pair_no][lambda_no] += math.log(factor)
            cumulative_differences[pair_no] += z

        pair_log_wealths = [
            _logsumexp(log_gamblers[pair_no]) - math.log(len(lambdas))
            for pair_no in range(len(pairs))
        ]

        for order_no, pair_indices in enumerate(order_pair_indices):
            if not alive[order_no]:
                continue
            log_order_wealth = (
                _logsumexp([pair_log_wealths[index] for index in pair_indices])
                - math.log(len(pair_indices))
            )
            max_log_order_wealth[order_no] = max(
                max_log_order_wealth[order_no], log_order_wealth
            )
            if max_log_order_wealth[order_no] >= threshold:
                alive[order_no] = False

        surviving = [order for order, is_alive in zip(orders, alive) if is_alive]
        if not surviving:
            raise RankEvaluationError("all weak orders eliminated; implementation or input invalid")

        canonical_row = _canonical_json({
            "episode_id": item["episode_id"],
            "active_arms": active_arms,
            "scores": {arm: float(scores[arm]) for arm in active_arms},
        })
        transcript_chain = hashlib.sha256(transcript_chain + canonical_row).digest()

        report = _rank_report(
            surviving, arms, pair_index, pair_log_wealths, alpha
        )
        retirement_decisions = _select_retirements(
            report, active_arms, retirement_rule, top_k_target
        )
        retired_this_look = [row["arm"] for row in retirement_decisions]
        for row in retirement_decisions:
            retirement_log[row["arm"]] = {
                "look": t,
                "rule": row["rule"],
                "evidence": row["evidence"],
            }
        next_active_arms = [
            arm for arm in active_arms if arm not in set(retired_this_look)
        ]
        frozen_pairs = [
            f"{arms[left]}>{arms[right]}"
            for left, right in pairs
            if arms[left] not in next_active_arms or arms[right] not in next_active_arms
        ]
        if not report["shortcut_is_subset_of_exact"]:
            raise RankEvaluationError("shortcut certified a dominance absent from exact enumeration")
        pair_log_wealth_report = {
            f"{arms[left]}>{arms[right]}": pair_log_wealths[pair_index[(left, right)]]
            for left in range(model_count)
            for right in range(model_count)
            if left != right
        }
        looks.append({
            "look": t,
            "episode_id": item["episode_id"],
            "active_arms": active_arms,
            "score_vector": {arm: float(scores[arm]) for arm in active_arms},
            "pair_log_wealths": pair_log_wealth_report,
            **report,
            "retirement_rule": retirement_rule,
            "retirement_decisions": retirement_decisions,
            "retired_this_look": retired_this_look,
            "next_active_arms": next_active_arms,
            "frozen_pairs": frozen_pairs,
            "model_invocation_counts": dict(invocation_counts),
            "remaining_episode_opportunities": {
                arm: total_episodes - invocation_counts[arm] for arm in arms
            },
            "transcript_chain_sha256": transcript_chain.hex(),
        })
        expected_active_arms = next_active_arms
        if not expected_active_arms:
            completion_status = "ALL_MODELS_RETIRED"
            if t != len(items):
                raise RankEvaluationError("items cannot continue after all models have retired")
            break
    else:
        if len(looks) == total_episodes:
            completion_status = "COHORT_EXHAUSTED"
        else:
            completion_status = "INCOMPLETE_INPUT"

    return {
        "benchmark_id": BENCHMARK_ID,
        "method_id": METHOD_ID,
        "protocol_id": protocol_id,
        "metric_id": metric_id,
        "arms": arms,
        "sampling_model": "finite_cohort_random_order",
        "retirement_rule": retirement_rule,
        "retirement_top_k": top_k_target,
        "alpha": alpha,
        "betting_fractions": lambdas,
        "episode_count": total_episodes,
        "episode_order": episode_order,
        "episodes_processed": len(looks),
        "unprocessed_episode_count": total_episodes - len(looks),
        "completion_status": completion_status,
        "active_arms": expected_active_arms,
        "model_invocation_counts": invocation_counts,
        "model_invocations_avoided_vs_full": {
            arm: total_episodes - invocation_counts[arm] for arm in arms
        },
        "retirement_log": retirement_log,
        "seed_commitment": seed_commitment,
        "order_commitment_sha256": order_commitment,
        "score_transcript_sha256": transcript_chain.hex(),
        "weak_order_count": len(orders),
        "looks": looks,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True)
    parser.add_argument("--output")
    args = parser.parse_args()
    try:
        with open(args.input, "r", encoding="utf-8") as source:
            document = json.load(source)
        result = evaluate_document(document)
        rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
        if args.output:
            with open(args.output, "w", encoding="utf-8") as destination:
                destination.write(rendered)
        else:
            print(rendered, end="")
    except (OSError, json.JSONDecodeError, RankEvaluationError) as exc:
        print(json.dumps({"status": "INVALID", "error": str(exc)}), file=sys.stderr)
        raise SystemExit(2) from exc


if __name__ == "__main__":
    main()
