# SAFRB benchmark tournament — v0.2

## Evaluation philosophy

We do not claim superiority by comparing Symthaea to a weak baseline or by counting tests. Every tournament must publish:
1. a frozen dataset and split;
2. source and raw-data digests;
3. model/configuration/software versions;
4. training and inference compute budgets;
5. the full scoring script;
6. confidence intervals and subgroup/tail results;
7. negative controls and failed runs;
8. independent reproduction instructions.

Use temporal splits. Never randomly leak future observations into training. Report all features available at decision time; no future customs declarations, cargo milestones or inventory updates can leak into a historical forecast.

## Track A — Demand and imports forecasting

**Baselines**
- Seasonal naive (same period last year / seasonal lag).
- Damped exponential smoothing and ETS.
- ARIMA/SARIMAX where seasonality/covariates are justified.
- Gradient-boosted trees (XGBoost or LightGBM).
- Temporal convolutional network or DeepAR-class probabilistic model.
- Transformer time-series model, only when data volume supports it.
- Human planner / current operator forecast where a pilot partner permits.

**Metrics**
- WAPE and MASE (plus MAE/RMSE; do not use MAPE alone where actuals approach zero).
- Pinball loss at p10/p50/p90 and weighted interval score.
- Empirical interval coverage and calibration by horizon.
- Under-forecast magnitude and shortage-event recall.
- Performance by grade, month, province, disruption regime and source-quality tier.

**Pre-registered target:** ≥10% relative improvement in WAPE over the strongest eligible baseline on the primary horizon, while maintaining nominal interval coverage within a pre-agreed tolerance and not worsening p90 under-forecast error. This is a research gate, not a current result. With short or non-stationary histories, strong simpler models may remain better.

## Track B — Evidence and inventory reconciliation

**Baselines**
- Ordinary ERP tank roll-forward.
- SQL reconciliation rules with signed event logs.
- Event-sourcing with idempotency keys and custody references.
- GS1 EPCIS 2.0.1-compatible event exchange.
- W3C Verifiable Credentials 2.0 credential verification.
- Mycelix provenance adapter as an additional verifier, not the source of truth.

**Metrics**
- Unexplained volume (litres) and unexplained percentage.
- False-positive verified usable litres (hard safety metric; target exactly zero in benchmark).
- False rejection and unresolved volume.
- Duplicate/replay detection rate.
- Late/out-of-order event handling.
- Time to identify a reconciliation break.
- Independent verification latency and proof size.
- Sensitivity to stale calibration, tank strapping errors, density/temperature correction and product-grade mismatch.

No cryptographic signature proves that a sensor was correctly installed, calibrated or sampled the right tank. Sensor metrology and physical audit remain separate.

## Track C — Network allocation and resilience

**Baselines**
- Greedy priority dispatch.
- Maximum flow / minimum-cost flow.
- Linear programming for deterministic capacity-constrained allocation.
- Mixed-integer programming for discrete routes, batch windows and infrastructure availability.
- Two-stage stochastic programming with scenario recourse.
- Distributionally robust / robust optimisation.
- Current human logistics plan, where an operator can provide a comparable and anonymised case.

**Objective is multi-dimensional.** Report cost and emissions separately from:
- critical-demand service rate;
- total unserved demand;
- worst-zone and p95/p99 shortfall;
- CVaR95 of shortage volume / shortage cost;
- time to recover;
- number of customers/regions affected;
- supply concentration and single-point-of-failure exposure;
- fairness constraints and transparent priority rules.

**Benchmark protocol:** identical nodes, capacities, cargo events, grade compatibility, outage sets, time horizons and compute limits. Paired scenario seeds. Compare feasible solutions only; count runtime, optimality gap and solver timeouts. Publish infeasible and failed scenarios. Do not compare Symthaea's recommendation quality to a MILP planner unless both receive exactly the same decision-time state and objective.

**Initial target:** reduce CVaR95 of unserved critical demand by ≥10% at equal or lower total cost on a preregistered scenario suite, without worsening any hard safety/quality constraint. This target only becomes meaningful when a real or validated synthetic topology exists.

## Track D — Causal diagnosis and metrology

**Method baselines**
- Deterministic engineering rules and alarm rationalisation.
- PCA / PLS and statistical process monitoring.
- Random forest / gradient-boosted classifier.
- LSTM / TCN / Transformer temporal model.
- Industrial foundation model or vendor system, if a permitted matched evaluation is available.
- Symthaea HDC/CfC/LTC representation plus explicit causal hypotheses.

**Metrics**
- Event-level precision/recall and macro-F1.
- False alarms per asset-day.
- Mean and p95 detection delay.
- Root-cause top-k and causal edge precision/recall.
- Brier score / calibration error.
- Abstention coverage-risk curve.
- Information gain per follow-up observation.
- Ability to identify a disconfirming test rather than merely explain the observed alarm.

A benchmark must include sensor drift, missingness, asynchronous samples, regime changes and common-cause faults. AI4I and Tennessee Eastman can qualify the harness, but do not establish petroleum field performance.

## Track E — Physical network model

v0.2 is an accounting oracle, not a hydraulic model. A subsequent hydraulic lane must compare against an independent reference solver and analytical limits.

- Mass balance and grade balance.
- Pressure/flow state error.
- Pump power and energy balance.
- Transit/arrival time error.
- Batch interface / contamination volume.
- Tank capacity and overflow.
- Grid/time-step convergence.
- Numerical stability under stiff and discontinuous conditions.
- Uncertainty coverage for calibrated parameters.

Only publish solver claims for the exact geometry, fluid properties, boundary conditions, regime and operating envelope that passed.

## Universal negative controls

Every lane must include:
- wrong unit (L vs m³; bbl vs L);
- wrong grade (D50 vs D500);
- stale inventory timestamp;
- duplicate event and replayed cargo receipt;
- nominated/arrived cargo promoted to usable without discharge;
- off-spec or inaccessible tank volume;
- forged signer / expired credential;
- out-of-order custody event;
- correlated sensor failure / common-cause outage;
- one broken route and one shared upstream bottleneck;
- demand spike and supplier concentration shock;
- missing or contradicted source series.

## Reporting

Publish mean and tail metrics, uncertainty, subgroup results, failures, compute cost, carbon/energy cost of computation and independent reproduction status. A benchmark pass means the test contract was met; it does not imply operational readiness.
