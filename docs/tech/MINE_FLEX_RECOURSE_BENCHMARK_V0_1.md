# MINE-FLEX-RECOURSE-001 — causal adaptive dispatch recourse v0.1

## Goal

Test whether period-by-period recourse can recover flexibility lost by a static robust schedule without violating electrical capacity, protected water reserves, production constraints or decision-time information boundaries.

This composes the exact dispatch accounting and the finite uncertainty generator. It is not a production controller or a site-calibrated mine model.

## Policy and information contract

At period t, the adaptive policy receives:
- the published forecast;
- the frozen uncertainty envelope;
- qualified resource observations through the current period;
- batches already committed/executed in earlier periods.

It does **not** receive the hidden realization ID, seed, future realized periods or the evaluator's per-world optimum. It commits an integer batch count for the current period and cannot revoke earlier batches.

Each envelope scenario that matches the observed prefix contributes its future resource projection. If an observed prefix matches no member of the envelope, the support-miss fallback uses all declared future projections after overwriting only the periods already observed. The policy must then find a completion that stays feasible for every remaining projection or explicitly return a no-feasible-recourse result. It must not fabricate a probability distribution or claim the out-of-envelope condition is covered.

For each candidate current action, the exact tiny oracle finds the cheapest feasible continuation in every still-consistent world. The policy chooses the action that minimizes the worst of these continuation costs, then worst continuation carbon, then current-period batch count. The perfect-information realized-world schedule remains evaluator-only.

## Frozen world set

The generator and seed split are inherited from MINE-FLEX-ROBUST-001:
- validation seeds 1–32;
- held-out generated seeds 10001–10032;
- five in-envelope worlds per seed: nominal, P0 grid tight, P1 renewable shortfall, P1 water shortfall, and correlated P0/P1 stress with P2 price/carbon spike;
- three evaluator-only out-of-envelope challenges per seed: P1 water availability below the declared envelope (but above reserve), P1 water at its protected-reserve floor with simultaneous renewable loss, and a P2 last-moment water-reserve stress.

The first five worlds are the policy's declared envelope. The three challenges are hidden from policy selection and are used to test recourse and fail-closed behavior. The resulting total is 320 in-envelope + 192 out-of-envelope world outcomes = 512, split evenly by seed range.

The out-of-envelope challenge set is public, deterministic and authored—not blind external validation. Future work should add withheld shock types and a true evaluation freeze.

## Exact comparisons and scorecard

Compare the same realized world using:
1. fixed baseline schedule [2,1,1];
2. exact nominal forecast cost-minimizing schedule;
3. static bounded-minimax schedule from the declared envelope;
4. causal adaptive recourse.

For each split and world class, report feasible/infeasible outcomes, feasibility rate, mean cost and carbon among fully feasible completed schedules, mean cost regret, worst regret, nearest-rank P95 regret, worst-5%-mean regret, and adaptive incurred costs for partially executed trajectories. Infeasible production completion is never assigned artificial finite regret; it is counted separately. The evaluator's oracle is only a reference bound.

## Required invariants

- Zero safety-electricity, grid-capacity and protected-water violations among feasible actions/schedules.
- 100% completion of the production target in every in-envelope world for this frozen tractable fixture.
- Exact independent reconstruction of every completed schedule and of incurred-prefix resource accounting.
- Zero non-anticipativity violations: identical observed prefix must lead to identical action.
- Future-only perturbation must not alter the earlier decision prefix.
- Out-of-envelope shock detection/fallback must not be reported as success unless the complete production target was actually met and the independent resource checks pass.
- Same inputs, seed split, generator and source commit must reproduce the same receipt; the output binds the generated evaluation-set digest and source commit.

These are software and finite synthetic-set invariants, not a real-mine safety proof.

## Research motivation

- Wang et al. (2025), Distributionally robust hybrid energy management in smart mining: https://www.nature.com/articles/s41598-025-11013-x
- Integrated optimal operation of power and water systems under uncertainty: An adjustable robust optimization approach (2026): https://doi.org/10.1016/j.watres.2026.125325
- Unified robust–stochastic dispatch for reliable net-zero microgrids under PV uncertainty (2026): https://doi.org/10.1016/j.apenergy.2026.128113
- Li et al. (2026), Unlocking lithium mine load flexibility: https://doi.org/10.1016/j.energy.2026.141872

The numerical claims in those papers are not used as expected values for this synthetic corpus.

## Current limitations and next stage

Only three periods and integer process batches are represented. There are no physical network equations, equipment ramp/start limits, dynamic stockpiles, storage state, water quality, process kinetics or measured site disturbances. The finite envelope is not a guarantee against unspecified shocks. The next tier must introduce withheld shock families, confidence intervals across broader independent seeds, calibrated temporal response models and rights-cleared site-specific evidence.

## Ownership

Reuse the MINE-FLEX-001 accounting primitives, MINE-FLEX-ROBUST-001 generator and metrics, MINE-ADAPTIVE-001 temporal semantics, ORE-FEED-BLEND-001 blend feasibility and MINE-SYSTEM-001 campaign evaluator. Do not create another general optimizer or authoritative mine ledger.
