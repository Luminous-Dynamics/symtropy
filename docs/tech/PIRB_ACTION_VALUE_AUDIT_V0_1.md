---
title: PIRB Capacity-Constrained Action Value Audit
version: 0.1
status: research-stage contract
authority: MeasurementOnly
---

# PIRB capacity-constrained action-value audit v0.1

## Purpose

Forecast accuracy and response policy value are different problems. An early-warning system may rank event risk well yet allocate scarce response capacity inefficiently when expected harm, action costs and assumed intervention effects vary across cases. Conversely, an expected-utility policy can look attractive under assumptions that are not empirically justified.

PIRB compares three transparent policies under the same capacity:
- `no_action`;
- `risk_ranked_top_k`, selecting the highest issued probabilities up to K;
- `expected_net_benefit_top_k`, selecting the highest positive expected net-benefit cases up to K.

The scenario grid pre-registers capacities, action-cost multipliers and intervention-effectiveness multipliers before evaluation. All policies use the same forecast-issued case pool within a scenario.

## Research grounding and limitations

The Barcelona School of Economics' *Dynamic Early Warning and Action Model* explicitly separates an internal armed-conflict forecasting module from a decision module modelling preventive action: https://bse.eu/research/working-papers/dynamic-early-warning-and-action-model. *If You Only Have a Hammer: Optimal Dynamic Prevention Policy* studies how forecasts interact with preventive and de-escalating actions in a multistage model: https://bse.eu/research/working-papers/if-you-only-have-hammer-optimal-dynamic-prevention-policy.

These are design references, not an assumed PIRB performance target. Their conclusions depend on the model's damage/cost assumptions and identifying conditions. Do not compare headline monetary estimates across systems unless target definitions, cost units and assumptions match.

## Input contract

The Draft 2020-12 schema is `docs/schemas/pirb-action-value-v0.1.schema.json`; the example is `docs/fixtures/pirb-action-value-synthetic-v0.1.json`. The fixture contains invented probabilities, harm units, costs, effectiveness assumptions and outcomes, with placeholder digests.

A registration pins:
- one target estimand, forecast origin, target period, and cutoff for action-policy features;
- the forecast artifact, frozen outcome snapshot and action-assumption digest;
- a common value unit for expected harm and action cost;
- strictly increasing capacity/cost/effectiveness grids and a minimum group-reporting denominator.

Action features must be cut off strictly before forecast origin. The target period must follow the origin and the frozen outcome cutoff cannot predate the target. Every row must match the one registered target. Unknown and immature outcomes stay null; abstained forecasts must have null served probability and are never eligible for action allocation. The offline candidate probability remains for audit but is not treated as a served answer.

Historical scope requires authority status `authorized`, disclosure status `approved`, independently verified source capture, both governance references, and non-placeholder forecast/outcome/action-assumption digests. These fields still require external verification; strings alone are not proof of authorization.

## Policy arithmetic

For issued case i and a registered scenario:

- Model-expected harm without action is (p_i H_i).
- Scenario effectiveness is (e_i' = min(1, e_i m_e)), where (e_i) is the registered assumed effect and (m_e) the sensitivity multiplier.
- Model-expected harm avoided under the assumption is (p_i H_i e_i').
- Assumed action cost is (c_i m_c), where (c_i) is the registered cost and (m_c) the scenario cost multiplier.
- Model-expected net benefit is (p_i H_i e_i' - c_i m_c).
- Model-expected residual harm on the issued pool is (sum_{	ext{issued}} p_iH_i - sum_{	ext{selected}} p_iH_ie_i').

The risk-ranked baseline selects by issued probability, highest first with case-ID tie-breaking. The net-benefit policy ranks by assumed net benefit, highest first, and only selects cases with positive expected net benefit. Both obey the same K-action capacity. No-action selects nothing.

The expected-harm calculation is scoped to the **forecast-issued pool**; abstained cases are counted and displayed separately, not imputed as zero risk. The receipt also reports known events and observed event capture among selected cases using mature labels, but this is a retrospective allocation diagnostic, not an intervention-effect estimate.

## Receipt

`pirb-action-value-audit` emits a digest-bound aggregate JSON receipt with:
- forecast/outcome/assumption digests, source commit, target and outcome cutoff;
- every pre-registered sensitivity scenario and all three policy results;
- selected actions, unallocated capacity, expected avoided harm under assumptions, costs, net benefit and residual model-expected harm;
- total mature outcomes/events, selected-case event capture/precision and unknown/immature counts;
- group allocations subject to the registered minimum group count;
- explicit non-claims and no live-decision authority.

No case-level predictions or outcomes are copied into the receipt. Ties are deterministic. Every sensitivity scenario is emitted; no best-looking scenario is selected after evaluation.

## Acceptance benchmarks

1. Invalid target, label status, capacity grid, feature cutoff and served-probability/abstention contracts fail closed.
2. Action count never exceeds registered capacity; abstained cases never receive an action.
3. Formula-level expected-value outputs match a golden arithmetic oracle.
4. For each scenario, expected avoided harm minus assumed cost equals the reported model-expected net benefit, and baseline expected harm minus assumed avoided harm equals residual expected harm on the issued pool.
5. All policies use the same issued pool, cost/effect assumptions and capacity.
6. Every pre-registered scenario is reported, including negative net-benefit risk-ranked policies and sensitivities where the policy ordering changes.
7. Observed event capture and precision are reported separately from model-expected utility; group allocation is suppressed below the declared minimum.

## Claim ceiling

This is a **decision model under registered assumptions**, not an optimizer fitted to real outcomes and not a causal-inference design. The intervention effectiveness values are assumptions, not identified treatment effects. Expected harm avoided is not observed harm avoided. The audit does not run live dispatch, target individuals, withhold services, or authorize action. A causal impact claim requires separate evidence addressing confounding, action selection, spillovers, changing outcomes and intervention compliance.

No real data enters public CI.

## Reproduction

```sh
cargo run --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml \
  --bin pirb-action-value-audit -- \
  --input docs/fixtures/pirb-action-value-synthetic-v0.1.json \
  --source-commit "$(git rev-parse HEAD)"
```

## Dependencies

Follow-on to #2159, #2160, #2188, #2191 and #2275, and stacked after selective forecast audit PR #2272. Comparison work builds on frozen outcomes (#2109, #2118), comparability (#2170) and hierarchy consistency (#2190).
