# ENERGY-DECISION-VALUE-001: Forecast error versus realized dispatch value v0.1

## Hypothesis

Forecast ranking by a symmetric error statistic need not rank forecasts by critical-service outcomes. A slightly lower average error can be obtained by underpredicting a high-reserve tail case, while a more conservative forecast with higher error can preserve essential service. Evaluate forecasting twice: as an error against the declared target, and through the decisions and service outcomes it produces.

## Frozen synthetic experiment

- Four fixed three-interval futures, one two-quantum battery, one-quantum discharge limit.
- The only discretionary decision is whether to spend one quantum on flexible service now or hold enough energy for future essential demand.
- Forecast target: future essential-reserve quanta needed during grid-unavailable intervals.
- Forecasts are keyed to a declared observable context, never evaluator-only scenario IDs.
- No probability model; the four-row MAE is an unweighted finite-panel diagnostic, not an expected error or statistical estimate.

Known answer:

| Policy / forecast | Finite-panel MAE | Essential unserved | Flexible unserved |
|---|---:|---:|---:|
| Lower-MAE, tail-blind forecast | 0.25 | 1 | 0 |
| Higher-MAE, critical-reserve forecast | 0.50 | 0 | 2 |
| No-forecast conservative hold | not scored | 0 | 4 |
| Perfect-information upper bound | 0 by construction | 0 | 1 |

The perfect-information case is non-deployable and is reported only as an upper bound. The example is a deliberately constructed counterexample—not a claim that one forecasting method generally wins.

## Acceptance gates for Rust implementation

1. Exact fixture and report SHA-256 binding.
2. Validate all four scenarios, their chronological intervals, observable-context mappings, nonnegative demand and two-quantum storage constraints.
3. Ensure controller decisions use forecasts keyed by observable context; no access to scenario IDs or realized future truth.
4. Reproduce MAE 0.25 versus 0.50 and the service metrics above exactly.
5. Preserve each interval's action, SoC, discharge and service gaps.
6. Keep forecast error, essential/flexible service, policy regret, physical feasibility and economic metrics separate.
7. Do not compute probability-weighted cost, EVPI, VSS, CVaR, confidence intervals, or generalized performance claims.
8. Keep physical dispatch audit NOT_RUN for the abstract model; later couple only schedules accepted by the independent Sol-Atlas verifier.

## Paradigm shift

Stop evaluating forecasts solely as predictors. Evaluate forecasts as inputs to a fully specified decision policy under the same scenarios and constraints. Report forecast quality, service quality, regret and resource use independently; a better score on one axis must not erase a worse result on another. Add delayed/stale forecasts, correlated forecast errors, value-of-information query budgets, and out-of-distribution contexts only after the base fixture is exact.

## Industry comparison

Current literature includes decision-aware compound-risk forecasts for microgrid scheduling and out-of-sample operational-value evaluation for building load forecasts. A credible comparison must match time horizon, observable inputs, data split, storage dynamics, rates, service constraints and optimization budget. This synthetic fixture demonstrates the test pattern, not superiority over REopt, SAM, PyPSA or a commercial forecaster.

References:
- NLR REopt capabilities: https://www.nlr.gov/reopt/about/capabilities
- PyPSA stochastic optimization and non-anticipativity: https://docs.pypsa.org/latest/user-guide/optimization/stochastic/
- Decision-aware compound-risk forecasting for microgrid dispatch: https://www.sciencedirect.com/science/article/pii/S030626192601682X
- Forecast error versus operational value across building load forecasts: https://www.sciencedirect.com/science/article/abs/pii/S037877882601399X
