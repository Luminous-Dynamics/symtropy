# ENERGY-INFORMATION-FRESHNESS-001: Freshness and decision latency v0.1

## Question

A forecast has no decision value before it is available, and a numerically plausible stale forecast can be unsafe when treated as current. Test forecast publication time and validity windows as part of the controller's decision input, not as cosmetic provenance metadata.

## Frozen synthetic known answer

Four 3-interval futures, initial/capacity battery state of charge 2 quanta, max discharge 1 quantum per interval. The t0 decision is to serve one flexible quantum or hold reserve for essential service. Forecast lookup is keyed by observable context, never evaluator-only scenario ID. There is no probability model.

| Controller | Qualified? | Essential unserved | Flexible unserved |
|---|---|---:|---:|
| Fresh context forecast available at t0 | yes | 0 | 1 |
| Delayed forecast, fail-closed | yes | 0 | 4 |
| Stale forecast, fail-closed | yes | 0 | 4 |
| Blindly consume stale forecast | **no** | 1 | 0 |

The stale counterfactual obtains extra flexible service by spending reserve and causes one essential quantum to go unserved in the persistent-critical scenario. The accepted controller refuses to use stale data. This is an intentionally constructed finite-domain result, not a probability estimate or real-site guarantee.

## Rust acceptance gates

1. Bind exact raw panel and experiment bytes with SHA-256.
2. Validate issued-at, available-at and valid-until at the decision boundary; a source is usable only when `issued_at <= now <= valid_until` and `available_at <= now`, with a non-stale status.
3. When no forecast is usable, reserve full capacity and do not serve discretionary flexible load at t0.
4. Preserve complete per-scenario/per-interval action, SOC, discharge and service-gap traces.
5. Make the stale-blind case explicitly `DISQUALIFIED_UNSAFE_COUNTERFACTUAL`, regardless of its flexible-service result.
6. Reject altered source bytes, reordered scenarios or intervals, contradictory truth labels, unknown contexts, future-dated forecasts and unsupported probability weights.
7. No expected cost, EVPI, VSS, CVaR, confidence intervals or probabilities.
8. Mark physical dispatch audit `NOT_RUN` until the independent Sol-Atlas verifier evaluates a translated schedule.

## Paradigm shift

The information state is itself safety-relevant. A controller must reason about **what was knowable when it acted**, not what the record now says was true. Forecast publication, revision history, validity window, source identity and context mapping should be first-class input fields. A forecast that improves a score but arrives after the decision cannot be credited with that decision's success.

## Promotion ladder

Exact synthetic temporal-integrity gate → stale-age and delayed-update sweeps → revision history and conflicting-source tests → chronological held-out energy data → matched task-oriented forecasting and dispatch baselines. Report essential/flexible service, error, regret, freshness violations, runtime and memory separately.

## References

- Value-oriented renewable-energy forecasting, IEEE Transactions on Smart Grid: https://ieeexplore.ieee.org/document/10771620
- Forecast error and microgrid operational value, IEEE RTSI 2024: https://ieeexplore.ieee.org/document/10761823/
- PyPSA stochastic optimization/non-anticipativity: https://docs.pypsa.org/latest/user-guide/optimization/stochastic/
- NLR REopt capabilities: https://www.nlr.gov/reopt/about/capabilities
