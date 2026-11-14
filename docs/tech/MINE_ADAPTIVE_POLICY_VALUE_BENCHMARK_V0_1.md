# MINE-ADAPTIVE-POLICY-VALUE-001 — sequential policy tournament v0.1

## Purpose

Test whether sequentially chosen measurements and later replanning can outperform a static plan and a two-stage plan in a hand-checkable synthetic problem. This composes with the causal trace contract [PR #2053](https://github.com/Luminous-Dynamics/symtropy/pull/2053), measurement selector [Symthaea #8068](https://github.com/Luminous-Dynamics/symthaea/pull/8068), and campaign evaluator [Symtropy #1934](https://github.com/Luminous-Dynamics/symtropy/pull/1934). It does not replace them.

## The four policies / bounds

1. **Static plan:** select the prior-best lot and do not assay.
2. **Two-stage recourse:** assay lot A before route choice; use a predeclared rule based on A's delivered result.
3. **Rolling horizon:** assay A; if A is low, conditionally request the B assay; replan only after B's result arrives.
4. **Clairvoyant upper bound:** evaluator-only selection using both grades in advance and no information cost. It is not a deployable policy or a fair real-time competitor.

Policy functions receive only the prior route and observations already delivered. The simulator alone uses a hidden synthetic world's grade states to generate assay results and score realized utility. The assay result is scheduled for the following epoch, so a sample taken at epoch 0 cannot affect the epoch-0 decision.

## Synthetic utilities, deliberately dimensionless

The fixture uses four equally weighted worlds: A high/B high; A high/B low; A low/B high; A low/B low. Lot utility values are A-high=120, A-low=60, B-high=140, B-low=20. Each assay costs 2 utility units. These invented units are not dollars, grades, recoveries, or assay/campaign cost estimates.

Expected outcomes:
- Static plan: weighted-mean net utility 90; mean regret to the free-information upper bound 25.
- Two-stage recourse: weighted-mean net utility 98; mean regret 17.
- Rolling horizon: weighted-mean net utility 107; mean regret 8.
- Clairvoyant upper bound: weighted-mean utility 115.

In this intentionally constructed example, rolling horizon reduces mean regret by 52.94% versus the two-stage rule and 68% versus the static plan. This is a **known-answer regression fixture**, not an empirical estimate that rolling-horizon mining is generally 52.94% better.

## Validation gates

- All four worlds exactly match frozen net utility scores for static, two-stage, rolling-horizon, and clairvoyant upper-bound calculations.
- Expected means and regrets are recomputed using integer basis-point weighting; no floating-point rounding is used.
- Zero trace violations: any used event must appear in the decision snapshot and be delivered no later than the decision epoch.
- Zero non-anticipativity violations: for the same policy/epoch and identical visible-event snapshot, action outputs must be identical across worlds.
- Each decision records the full delivered-event snapshot and the dependencies it declares; the trace verifier checks snapshot completeness and dependency subset/time constraints.
- Result events are exposed only after their delivery epoch.
- Exact corpus SHA-256 and source-commit receipts are produced; unknown schema fields, duplicate scenario IDs, invalid probabilities, and unsupported assay delays fail closed.
- The clairvoyant bound is kept separate from policies and is never passed into their decision function.

## Research grounding

- Armstrong et al. (2021), *Adaptive open-pit mining planning under geological uncertainty*: rolling-horizon planning updates the geological model as information arrives. Its average advantage in the studied cases did not make it better on every simulated orebody; compare paired per-world regret rather than report only a grand mean. https://doi.org/10.1016/j.resourpol.2021.102086
- Khalifi et al. (2026 preprint), *Adaptive mine planning under geological uncertainty: A POMDP framework for sequential decision-making*, treats belief updates as part of sequential actions. Published gains require independent reproduction under matching assumptions. https://arxiv.org/abs/2605.13702
- Jeuken & Forbes (2024), *The Value of Drilling—A Chance-Constrained Optimization Approach*, motivates evaluating tests by downstream decision value net of constraints. https://doi.org/10.1007/s42461-024-01061-8

## Required next evidence tier

This four-world corpus is public and hand-authored. It cannot establish generalization. A later benchmark must use pre-registered independent generator code and held-out scenario seeds, compare policies on identical worlds with paired confidence intervals, model imperfect/noisy/out-of-order measurements, and report adverse-tail regret and constraint breaches. Any perfect-knowledge policy remains an upper bound. External model/data reproduction requires pinned assumptions, source permissions, and exact formulation alignment.

## Claim ceiling

This benchmark proves only a small policy-comparison known answer and trace discipline on synthetic data. It does not establish real mine performance, orebody inference, reserve quality, assay value, worker safety, or commercial value.
