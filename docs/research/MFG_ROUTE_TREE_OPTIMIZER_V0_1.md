# Manufacturing Route Scenario-Tree Optimizer v0.1

**Owner:** Symtropy issue #7084.  
**Implementation:** draft research branch `research/mfg-route-tree-optimizer-v0-1`.  
**Evidence class:** synthetic, offline.  
**Claim ceiling:** no provider, capacity offer, production route, product, quality result or physical output is qualified.

## Purpose

The route-recourse verifier #7053/#7083 checks non-anticipativity, observed route invalidation, unresolved-execution freezes and capacity-safe recommendations. Its exact oracle is exhaustive only for the current decision's assignment.

This increment addresses a different mathematical problem: choose an initial route preference and a contingent recourse policy across future observation branches, minimizing expected cost while requiring identical actions whenever the available information is identical.

This is a bounded **two-stage scenario-tree policy optimizer**. It is not a full multi-period production scheduler, reservation engine or learned policy.

## Ownership and composition

Reuse, do not replace:
- Symtropy #6790: route feasibility and hard blockers.
- Symtropy #7019/#7021: primary-route policy benchmark.
- Symtropy #7053/#7083: observation semantics, non-anticipativity, reconciliation and capacity verification.
- Symtropy #2439: scenario-family robustness.
- Symtropy #5758: intended/recorded/observed execution reconciliation.
- Mycelix #5762 and Symthaea #5774: common-cause closure.
- Symthaea #7677: scheduling.
- Mycelix #6541: resource-offer declarations, not reservations.
- Mycelix #3084: accepted manufactured-unit evidence.
- Symthaea #7706: machine-intervention authority.

The crate is an offline mathematical benchmark over synthetic route/capability inputs. It does not replace any of those owners, reserve capacity, create work orders, dispatch production or control machinery.

## Two-stage information model

**Stage 0 — initial preference.** Before disruption observations, select a primary route for the one admitted synthetic job. This is an initial recommendation/preference used by the optimization model; it is not a physical booking or dispatch.

**Stage 1 — observation and recourse.** The scenario tree reveals a route-specific outage, shared common-cause outage, calibration/route evidence becoming Unknown, capacity contraction, or a nominal state. For each distinct information set, the optimizer selects one recourse route or abstains. Every scenario sharing that information set receives the same recourse action.

Some leaves intentionally share the same observable information but differ in hidden terminal quality outcomes. The policy sees the common observation and chooses once; the evaluator sees the hidden terminal event and measures its cost. The exact policy optimizer may use the declared scenario distribution, but cannot choose different actions for indistinguishable histories.

### Objective

For each terminal scenario:
- production route cost × quantity;
- switch/replanning friction × quantity when recourse differs from the initial primary route;
- terminal quality-rejection penalty × quantity if the hidden outcome rejects the selected route;
- or unserved-demand penalty × quantity if the policy abstains.

The objective is evaluated as an integer probability-weighted numerator in parts per million (PPM), avoiding floating-point accumulation in optimization and known-answer checks.

Switch and rejection penalties are synthetic challenge parameters, not quotations or measured costs.

## Exact small-instance algorithm

For each possible initial primary route:
1. Partition terminal scenarios by canonical observed-information-set identity.
2. For each information set, enumerate every currently admissible recourse route plus abstention.
3. Compute the probability-weighted terminal cost for each candidate action across all scenarios in that information set.
4. Select the lowest-cost common action for that information set.
5. Sum all information-set expected costs.
6. Select the initial primary route with minimum total expected cost.

This is exact for the bounded one-job/two-stage model because recourse decisions at distinct terminal information sets are independent once the primary choice is fixed. It is not a generic multi-stage contingent-policy optimizer.

The report also computes a clairvoyant lower bound that may choose a different recourse action for each hidden leaf. That bound is diagnostic only and is never used as the primary policy comparator or pass/fail criterion.

## Frozen synthetic corpus

The corpus contains six scenario families, 20 paired seeds per family, 120 scenario trees, five terminal leaves per tree (600 leaves total), and four information sets per tree (480 information sets total).

Families:
1. nominal and local outage;
2. supplier variability;
3. common-cause outage;
4. maintenance/calibration uncertainty;
5. quality/inspection uncertainty;
6. demand surge and capacity contraction.

Each tree has two local-outage leaves with identical visible observations but different hidden terminal quality outcomes. The recourse action is shared between them. Route, domain, cost, lead-time, capacity and quality data are synthetic only.

## Baselines and outputs

The evaluator compares:
- **Greedy:** cheapest initially admissible route; keep it whenever still admissible; otherwise switch to the cheapest currently admissible route or abstain.
- **Exact non-anticipative policy:** optimize the root preference and one recourse action per information set.
- **Clairvoyant lower bound:** diagnostic leaf-perfect recourse, not an achievable policy.

The report includes all three probability-weighted cost numerators, selected policy plans, per-case regret, information-set counts and non-anticipativity status.

## Initial cross-language fixture findings

A separate JavaScript implementation recomputed every greedy plan, exact policy plan, expected-cost numerator and clairvoyant bound from the frozen corpus and known-answer fixture. It found zero mismatches across 120 cases.

| Measure | Synthetic result |
|---|---:|
| Scenario trees | 120 |
| Terminal leaves | 600 |
| Information sets | 480 |
| Mean greedy expected cost | 16.0292 units |
| Mean exact non-anticipative expected cost | 14.7825 units |
| Mean clairvoyant lower bound | 14.6825 units |
| Total greedy regret across 120 cases | 149.6 units |
| Cases where exact policy strictly improves on greedy | 40 / 120 |
| Trees with repeated information set and divergent hidden quality outcome | 20 / 120 |

These are challenge-fixture results, not empirical manufacturing outcomes. Cost values and penalties are synthetic; they do not represent ZAR, quotations, cost per accepted part, supplier failure rates or physical yield. The small gap between the clairvoyant lower bound and exact non-anticipative optimum is a property of this chosen fixture, not a general value-of-information bound.

## Research baseline and next algorithmic frontier

Relevant 2026 research:
- Kim and Jung, *On-demand supply chain configuration and planning for shared additive manufacturing* (2026), integrates provider selection, part-to-job grouping, job-to-machine assignment and daily scheduling using MILP and GA. https://doi.org/10.1080/00207543.2026.2728013
- Thakur and Mukherjee (July 2026), *Managing demand uncertainty and supply disruption risks*, uses hybrid scenario trees and rolling-horizon optimization. https://doi.org/10.1016/j.cie.2026.112047
- *Scheduling a flow shop with dual sources of uncertainty: a distributionally robust approach* (17 June 2026). https://doi.org/10.1080/0305215X.2026.2677056
- Alrashdan (26 August 2026), *Routing under machine breakdowns*, benchmarks dispatching rules, bandits and PPO; tuned simple queue rules remain important comparators and learned-policy results are sensitive to protocol. https://doi.org/10.1007/s44444-026-00128-9

The next mathematical step is a multi-period scenario tree with decisions at multiple stages, parent/child transitions, canonical information-set partitions at every stage, non-anticipativity constraints across all nodes sharing an observation history, time-expanded capacity, recourse after observed failures/inspection outcomes, and terminal accepted/rejected/late/unserved/unresolved states.

Compare deterministic plan-and-repair, rolling-horizon greedy, two-stage stochastic, multi-stage stochastic, distributionally robust policies, strong dispatching rules and exact tiny-tree oracles on identical scenarios and solver budgets. Add learned bandit/RL policies only after simple baselines are tuned and evaluation sets are protected from checkpoint selection.

## Proposed targets

These are hypotheses, not production results:
- 100% identical action for identical information sets;
- exact agreement with exhaustive known answers on the bounded two-stage cases;
- clairvoyant lower bound never above the non-anticipative optimum;
- zero selection of routes with observed blockers or Unknown status;
- deterministic replay for identical corpus, evaluator and policy;
- complete per-family reporting with whole-family holdouts for generalization claims;
- full solver settings, compute budgets and optimality gaps for larger experiments.

## Verification boundary

The crate uses strict Serde types and an offline Draft 2020-12 JSON Schema validator. Its focused workflow runs formatting, schema validation, known-answer/adversarial tests, exact policy evaluation, negative controls, hashing and strict Clippy.

The exact oracle is exact only for the stated one-job/two-stage formulation. It does not establish optimal multi-stage scheduling, physical recovery performance, supplier availability, capacity reservation, product quality, machine qualification or accepted production. All recommendations remain non-binding and all physical authority remains with existing owners.