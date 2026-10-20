---
title: PIRB Value-of-Information Audit
version: 0.1
status: research-stage contract
authority: MeasurementOnly
---

# PIRB value-of-information audit v0.1

## Purpose

The audit asks whether an information source is worth acquiring before a registered decision deadline. It compares the best expected utility under the current prior to the expected utility after observing the source's signal, then subtracts acquisition cost and the registered delay penalty.

This is a finite-state **expected value of sample information (EVSI)** calculation under a fully declared model. It is not a source-truth verifier, forecast evaluator, intervention effect estimator or causal-impact study.

## Paradigm shift: decision value, not entropy alone

Uncertainty and information utility are different. A source can have high signal entropy while its signals are independent of the latent state, in which case it does not change the best decision and has zero gross EVSI. Another source with a less entropic signal may materially change the action choice and have positive net value. The synthetic fixture intentionally demonstrates this: the random feed has higher signal entropy and lower cost, yet the decision-relevant briefing has positive net EVSI. The random feed does not.

This choice aligns with conflict-prevention research that explicitly links forecasting to preventive action and information gains, including [*If You Only Have a Hammer: Optimal Dynamic Prevention Policy*](https://bse.eu/research/working-papers/if-you-only-have-hammer-optimal-dynamic-prevention-policy) and the forecast-plus-decision architecture in [*Dynamic Early Warning and Action Model*](https://bse.eu/research/working-papers/dynamic-early-warning-and-action-model). It also connects to budgeted data-acquisition under distribution shift, e.g. [He et al., ICML/PMLR 2024](https://proceedings.mlr.press/v235/he24f.html). Those references motivate the design; they do not validate the synthetic prior, utilities, signal likelihoods or acquisition ranking used here.

## Exact finite-state model

The contract is `docs/schemas/pirb-voi-v0.1.schema.json`; the checked-in fixture is `docs/fixtures/pirb-voi-synthetic-v0.1.json`. Values are invented and aggregate-only.

Let states be (sinmathcal{S}) with registered prior (P(s)), actions (ainmathcal{A}), and utility (U(a,s)) in the registered common value unit. The current decision value is:

[
V_0 = max_{ainmathcal{A}} sum_{sinmathcal{S}} P(s)U(a,s).
]

For candidate source (q), signals (zinmathcal{Z}_q) have conditional likelihood (P(zmid s,q)). The marginal probability of a signal is (P(zmid q)=sum_s P(s)P(zmid s,q)), and the posterior is Bayes' rule:

[
P(smid z,q) = rac{P(s)P(zmid s,q)}{P(zmid q)}.
]

The expected decision value after acquiring the source is:

[
V_q = sum_{zinmathcal{Z}_q} P(zmid q)max_{ainmathcal{A}}sum_s P(smid z,q)U(a,s).
]

Gross EVSI is (V_q - V_0). Net EVSI is:

[
operatorname{NetEVSI}(q)=V_q-V_0-c_q-d_q
]

where (c_q) is registered acquisition cost and (d_q) is the explicit delay penalty. The v0.1 audit enumerates every possible signal and does not fit priors, utilities, likelihoods, costs or penalties from held-out outcomes.

The best expected-utility action uses deterministic lexical tie-breaking. Priors and each state's signal likelihoods must sum to 1 within the registered numerical tolerance. A source that contains a zero-marginal-probability signal is rejected rather than dividing by zero or silently discarding a signal.

## Timing and admissibility

A source must arrive strictly after the forecast origin and no later than the registered decision deadline. A late source is explicitly marked `arrives_after_decision_deadline` and receives null actionable EVSI values. The calculator will not recommend that a decision be delayed merely because a late signal would be informative.

For historical evaluation, the prior and source-likelihood fit cutoffs must both precede the forecast origin; each source must have status `authorized`, source capture must be independently verified, disclosure must be approved, and all source/model digests must be non-placeholder values. These fields are still declared claims that need external verification; schema conformance is not authorization.

## Baseline heuristics

The receipt compares:
- highest **positive net EVSI** source (none is selected if all admissible net values are non-positive);
- lowest acquisition-cost admissible source;
- highest marginal signal-entropy admissible source.

Every source is reported, including inadmissible sources and ones with negative net value. These heuristics are comparison baselines, not automatic acquisition authorization. An evidence source should be considered only when its expected decision value covers its cost and delay under the registered assumptions and the source passes governance review.

## Receipt

`pirb-voi-audit` reports:
- registered state/action/source counts, current best action and current expected utility;
- per-source admissibility, deadline status, cost/delay, signal entropy, gross/net EVSI and recommendation under the explicit utility model;
- every possible signal's marginal probability, best action, conditional expected utility and posterior SHA-256 commitment;
- the three source-ranking baselines;
- schema/input/model digests, source commit and a deterministic receipt commitment;
- explicit claim ceilings, including `entropy_is_decision_value=false`, `causal_impact_established=false`, and no live decision authority.

No state-by-case personal data or real-world conflict data are included in the receipt.

## Acceptance benchmarks

1. **Exact arithmetic:** the synthetic informative source has baseline expected utility 2, expected post-information utility 4.4, gross EVSI 2.4 and net EVSI 1.7 under the registered invented cost/delay assumptions.
2. **Uninformative source:** state-independent signals yield zero gross EVSI (within numeric tolerance), even if their signal entropy is high.
3. **No entropy fallacy:** the max-entropy and lowest-cost heuristics can select a source with negative net EVSI while the decision-value ranking selects a different positive-value source.
4. **Deadline:** a highly informative source arriving after the decision deadline is not actionable and gets no actionable EVSI recommendation.
5. **Normalization / support:** invalid priors, incomplete likelihood tables, likelihoods that do not normalize, duplicate state/action/signal/source IDs, unknown states and zero-probability signals fail closed.
6. **No leakage:** prior and signal-likelihood fit cutoffs must precede forecast origin.
7. **Governance:** historical sources require permission, disclosure and independently captured provenance.
8. **Reproducibility:** identical registration bytes/schema/source commit yield identical signals, posterior commitments and receipts.

## Claim ceiling

This tool evaluates a declared model. It does not establish that the source actually has the registered likelihoods, that priors/utilities/costs are calibrated, that acquired information improves real forecast accuracy, or that a policy reduces violence. Gross/net EVSI is assumption-conditioned expected decision value, not observed benefit or a causal effect. No live dispatch, person-level targeting or service-access gating is supported.

## Reproduction

```sh
cargo run --manifest-path tools/pirb-probability-forecast-eval/Cargo.toml \
  --bin pirb-voi-audit -- \
  --input docs/fixtures/pirb-voi-synthetic-v0.1.json \
  --source-commit "$(git rev-parse HEAD)"
```

## Dependencies

Follows #2281, #2275 and #2160, stacked after the selective forecast and action-value audit work (#2272, #2280), with comparable targets and frozen outcomes from #2170, #2118 and #2109.
