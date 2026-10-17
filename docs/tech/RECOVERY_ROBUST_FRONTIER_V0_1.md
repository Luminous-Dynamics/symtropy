---
title: Interval-Robust Pareto Frontier for Recovery Plans
version: 0.1
status: research-design
date: 2026-10-17
issue: https://github.com/Luminous-Dynamics/symtropy/issues/1874
---

# Interval-Robust Pareto Frontier for Recovery Plans v0.1

## Claim ceiling

This is an effects-disabled synthetic reference algorithm. It does not authorize felling, lifting, transport, repair, reuse, dismantling, or hazardous-material handling. The included values are invented test values, not calibrated estimates. Passing this audit means the frozen example agrees with the declared mathematical rule; it does not prove the rule is sufficient for real-world operations.

## Paradigm

Do not output one winner by hiding cost, energy, ecological effects, and uncertainty in an unexplained scalar. Return a **set of defensibly non-dominated feasible plans**, and attach a machine-checkable reason whenever one plan can be robustly preferred to another under the declared ranges.

The decision architecture separates:
1. Mandatory feasibility (permissions, safety, ecology, biosecurity, engineering authority, and other profile-defined gates).
2. Outcome intervals and units for each comparable objective.
3. Conservative dominance proof and Pareto frontier.
4. Human/authority review and any later execution authorization outside this tool.

A cheap plan with a failed safety gate remains infeasible, even if every objective estimate is attractive.

## V0.1 interval-separated dominance theorem

The corpus uses one versioned decision profile with a fixed set of **minimize** objectives. Each plan has inclusive integer bounds `[lower, upper]` for every objective with its own explicit unit. Nonnegative integer quanta avoid floating point and implicit unit conversion in the reference path.

For feasible plans A and B, certify A robustly dominates B iff:

```text
for every objective j:
    upper(A, j) <= lower(B, j)

and for at least one objective k:
    upper(A, k) < lower(B, k)
```

This is a conservative sufficient certificate under independent interval-box uncertainty: even A's worst declared outcome is no worse than B's best declared outcome in every minimize objective, and strictly better in at least one. The margin `lower(B,j) - upper(A,j)` is emitted per objective with the original unit.

Important consequences:
- Overlapping uncertainty intervals usually mean **no dominance certificate**, not that the plans have equal value.
- Equal interval vectors do not strictly dominate each other.
- A new objective where A can be worse than B can invalidate a previous A→B certificate. Adding dimensions is not a harmless metadata change.
- Widening A's upper bound or lowering B's lower bound cannot create a stronger A→B certificate.
- The frontier can contain several alternatives. A separate, explicit human preference policy can choose among them if applicable; this comparator does not invent weights.
- This theorem is deliberately conservative and does not model correlations between objectives, dependencies among scenarios, probability distributions, maximum regret, or maximize objectives. Those need separate schemas and tests.

## Gate semantics

Every plan has one or more uniquely named mandatory gates. Status is exactly `PASS`, `BLOCK`, or `UNKNOWN`. A plan is feasible only if all gates are `PASS`. A plan with a `BLOCK` or `UNKNOWN` gate is excluded before objective comparison. Estimated cost or recovery value cannot override gate status.

In production, a complete alternative set should include safe hold / retain / inspect / refer-to-specialist options as domain policy permits. The synthetic six-plan case exists to test comparator semantics; it is not a list of authorized timber operations.

## Frozen corpus

`recovery-robust-frontier-corpus-v0.1.json` supplies six synthetic alternatives and three objective dimensions:
- synthetic full-chain cost per qualified output;
- synthetic energy per qualified output;
- synthetic ecological disturbance points.

The expected robust frontier is:
- `fast_local_specialist`;
- `local_sorted_reuse`;
- `uncertain_remote_extraction`.

`remote_haul_and_process` is robustly dominated by `local_sorted_reuse`, which emits a per-objective margin certificate. The nominally very cheap `unsafe_quick_disassembly` is excluded by a `BLOCK` gate. `permit_unknown_airlift` is excluded by an `UNKNOWN` permission gate. A passing frontier audit is not a field recommendation.

## Benchmarks

V0.1 targets:
- **Zero gate leaks:** no `BLOCK`/`UNKNOWN` plan enters the feasible set.
- **Exact fixture agreement:** feasible set, frontier, and dominance-pair set match the frozen oracle exactly.
- **Input-order invariance:** plan and objective enumeration order do not change the semantic frontier.
- **Uncertainty monotonicity:** widening intervals must not create a newly justified dominance certificate.
- **Objective completeness:** adding a dimension can break dominance when that dimension introduces a trade-off; silently ignoring a dimension is a failure.
- **Unit/profile enforcement:** a mismatch rejects with a structured finding.
- **Receipt reproducibility:** identical canonical inputs/profile/version produce identical receipt contents; any input byte change changes the input digest.
- **No single-winner coercion:** non-dominated feasible plans remain available as a set.

After the reference behavior stabilizes, expand to at least 100 adversarial fixtures per domain profile as a proposed testing goal. Include ties, touching bounds, wide/overlapping intervals, single-objective and many-objective cases, missing or extra dimensions, duplicate IDs, gate conflicts, objective additions, objective reordering, plan permutation, and extreme integer bounds. Then compare against an independently written brute-force oracle. Synthetic logic tests do not establish real-world decision quality; field evaluation needs labeled cases, domain experts, and a preregistered evaluation plan.

## Research grounding

- Chen et al., *Cost-effective and reliable power system transitions under deep uncertainty: A multi-scenario multi-objective robust decision making framework* (April 2026, DOI 10.1016/j.ijepes.2026.111766). It distinguishes constraint-related from objective-related uncertainty and compares cost/regret/reliability across scenarios. Numeric findings belong to its power-system case, not this forest example: https://doi.org/10.1016/j.ijepes.2026.111766
- *A Systematic Review of Implementing Multi-Criteria Decision-Making Approaches for the Circular Economy and Cost Assessment* (2025, DOI 10.3390/su17115007): https://doi.org/10.3390/su17115007
- Review of adaptive planning methods emphasizes regret and satisficing across evolving scenarios: https://doi.org/10.1016/j.cacint.2025.100196
- Related Symtropy context: [#1850 resource-recovery decision oracle](https://github.com/Luminous-Dynamics/symtropy/issues/1850), [#1661 value of information](https://github.com/Luminous-Dynamics/symtropy/issues/1661), [#1769 robust design and survey planning](https://github.com/Luminous-Dynamics/symtropy/issues/1769), [#1777 evidence-derived constraints](https://github.com/Luminous-Dynamics/symtropy/issues/1777).

## Non-goals

- no global scalar score;
- no assumed probability distributions;
- no correlation-aware uncertainty model yet;
- no automatic action or permission;
- no empirical timber yield/cost/carbon claims;
- no claim that the frontier includes every operationally feasible plan unless candidate generation coverage is separately qualified.
