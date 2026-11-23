# Reliability-aware decision-relevant acquisition planner

**Status:** synthetic research wrapper composing the #7082 acquisition-reliability admission with the #7145 decision-aware planner. No physical sensing has been run.

## Why another planning dimension is needed

The decision-aware planner measures how much a proposed observation could separate hypotheses that imply different navigation decisions, conditional on a successful, valid observation.

That conditional value alone is not enough to rank actions. A high-value observation that rarely arrives fresh, valid and independently qualified may be inferior to a less discriminating observation with a much stronger admitted success-probability lower bound.

The new wrapper consumes #7082's externally computed, domain- and protocol-bound probability interval. It does not estimate that interval or replace the interval's method, campaign grouping or source-admission review.

## Reliability-adjusted value

For action a:
- let D(a) be the number of cross-decision hypothesis pairs whose conditional outcome sets are guaranteed disjoint;
- let [pL(a), pU(a)] be the admitted probability interval for a fresh, valid, independently qualified observation under the frozen protocol and domain.

Report the lower/upper planning-value interval:

expected decision-pair yield in [pL(a) × D(a), pU(a) × D(a)]

This is a lower/upper expected-yield calculation under the caller-supplied admitted probability contract. It is **not** an observed yield, a point probability, a deterministic guarantee or a physical validation result.

Also report the probability interval, confidence-error probability, independent campaign-block count and source/cohort/method commitments. Keep the following as separate axes:
- lower-bound expected decision-pair yield;
- lower-bound expected signed protection-level reduction;
- worst-case acquisition latency;
- worst-case energy.

The Pareto frontier retains trade-offs; no weighted scalar converts metres, pair counts, probability, time and energy into one score.

## Admission rules

The planner first applies the existing availability, freshness/calibration, dependency-group, maximum-latency and maximum-energy gates. It then passes only those resource-feasible action IDs to #7082.

Missing, unsupported-domain, protocol-mismatched, too-small lower-bound, too-weak confidence, or insufficient-independent-block contracts are rejected actions. They are never assigned probability 1. The #7082 admission also checks campaign-block identities, unique independence groups and that the supplied interval encloses the observed valid-success fraction.

The probability interval remains an external claim: #7082 checks structure and policy but does not recompute the statistical bound or prove the blocks physically independent.

## Matched selection logic

1. If the protection target is already satisfied, no acquisition is recommended.
2. Otherwise, each reliability-admitted action must also have a valid decision-equivalence report for the same complete hypothesis inventory and frozen decision partition.
3. The planner reuses #7145's validator and conditional assessment for each candidate, but recomputes the frontier after probability adjustment. It does not reuse the conditional-only frontier, because an action dominated conditionally can become Pareto-relevant after its success probability is included.
4. A target-reaching action is preferred by expected decision-pair yield, expected protection reduction, latency and energy.
5. If no action reaches the target, maximize lower-bound expected decision-pair yield; if no action can resolve a cross-decision pair, use lower-bound expected protection reduction as a separate fallback objective.
6. The selected action remains conditional on later acquisition, time applicability, independent posterior audit and the separate authority owner.

## Synthetic test cases

- A two-pair action with a low admitted probability lower bound loses to a one-pair action with a much stronger success lower bound.
- An action below the minimum probability lower-bound threshold is explicitly rejected.
- A missing reliability contract is not treated as probability 1.
- A mismatched acquisition protocol is excluded.
- An already-satisfied target requires no unnecessary reliability contract.
- Duplicate independence groups make the reliability contract structurally invalid.

## Claim ceiling

The wrapper's probability interval is externally supplied and must be method-reviewed. A campaign-block ID is not proof of statistical independence. The lower-bound expected decision-pair yield is useful for action ranking only under the probability contract's assumptions.

The wrapper does not execute sensors, verify a physical outcome model, update the posterior/protection level, grant NORMAL or establish PHMI, mission availability, physical containment, certification or SOTA superiority.
