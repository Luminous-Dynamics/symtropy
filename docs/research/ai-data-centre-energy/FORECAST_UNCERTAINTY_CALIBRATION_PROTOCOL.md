# Joint Forecast-Uncertainty Calibration Protocol

**Protocol:** AI-FORECAST-CALIBRATION-001  
**Version:** 0.1  
**Status:** research protocol; synthetic Rust fixtures only  
**Owner:** Symtropy energy-systems assurance

## 1. Purpose

A data-centre energy optimizer is only as credible as the uncertainty information on which it acts. Low point-forecast error does not establish calibrated uncertainty, and separately calibrated workload, renewable, price and temperature marginals do not establish simultaneous coverage of their combined future path.

This protocol evaluates a simultaneous forecast-error envelope over a frozen set of channels and horizons. It is an assurance input to the larger compute-power-cooling orchestration loop, not a forecast model, scenario generator, optimizer, or operational controller.

The Rust implementation is `crates/domains/symtropy-energy-systems/src/forecast_calibration.rs`. It composes with:

- workload schedule feasibility: [#3435](https://github.com/Luminous-Dynamics/symtropy/issues/3435) / [#3445](https://github.com/Luminous-Dynamics/symtropy/pull/3445);
- scenario-weighted empirical risk: [#3579](https://github.com/Luminous-Dynamics/symtropy/issues/3579) / [#3581](https://github.com/Luminous-Dynamics/symtropy/pull/3581);
- storage and thermal trajectory feasibility: [#3575](https://github.com/Luminous-Dynamics/symtropy/issues/3575) / [#3578](https://github.com/Luminous-Dynamics/symtropy/pull/3578);
- energy measurement and verification: [#3573](https://github.com/Luminous-Dynamics/symtropy/issues/3573) / [#3574](https://github.com/Luminous-Dynamics/symtropy/pull/3574).

These are separate owners and separate claim boundaries.

## 2. V0.1 calibration method

For forecast block b, channel c and horizon h, let y(b,c,h) be the realized value, f(b,c,h) the point forecast issued before the target window, and s(b,c,h) a declared positive normalization scale.

The block nonconformity score is the maximum scale-normalized absolute error over every declared channel and horizon:

    score(b) = max over c,h of abs(y(b,c,h) - f(b,c,h)) / s(b,c,h)

For n chronological calibration blocks and target joint coverage C represented in parts per million, the selected 1-indexed order statistic is:

    k = ceil((n + 1) * C)

V0.1 computes this rank with integer arithmetic. It does not interpolate. If k exceeds n, the evaluator returns `InsufficientCalibrationBlocks` and no finite multiplier is reported.

Let q be the selected calibration score. Every held-out channel/horizon cell receives the interval:

    [f(b,c,h) - q*s(b,c,h), f(b,c,h) + q*s(b,c,h)]

The block is jointly covered only when every declared cell is covered. This is intentionally stricter than averaging marginal coverage.

### Minimum calibration blocks for a finite order statistic

| Requested coverage | Minimum calibration blocks |
|---:|---:|
| 90% | 9 |
| 95% | 19 |
| 99% | 99 |

These are mathematical minima for a finite split-conformal order statistic, **not** adequate sample sizes for a strong empirical reliability claim.

## 3. Statistical assumptions and claim ceiling

The usual finite-sample split-conformal coverage statement requires exchangeability between calibration and future blocks for the chosen score construction. Time-ordered separation prevents look-ahead leakage; it does **not** make adjacent time blocks exchangeable or independent.

The evaluator therefore:

- enforces forecast issue time strictly before target validity;
- enforces non-overlapping calibration windows and non-overlapping evaluation windows;
- requires every evaluation window to begin at or after the final calibration window;
- requires the same complete channel-by-horizon grid in every block;
- rejects missing, duplicated or undeclared cells and missing evidence IDs;
- records observed held-out joint and cell coverage rather than asserting future coverage.

It does not prove exchangeability, eliminate autocorrelation, guarantee coverage under regime shift, estimate a copula, or validate the forecast scale. The report names the exchangeability assumption. An empirical holdout gate is a result on that holdout set, not a universal guarantee.

For real time series, block length and spacing must be preregistered using residual autocorrelation and system operating cycles. Adjacent non-overlapping windows can remain strongly dependent. Final reporting must include dependence-aware uncertainty for realized coverage and effective sample-size reasoning. Zhai, Cheng and Wu (2026), [Conformal Coverage of Time Series: Validity and Inference](https://arxiv.org/abs/2609.33868), examine realized-coverage inference and block-based standard errors. Barber and Pananjady (2026), [Predictive inference for time series: why is split conformal effective despite temporal dependence?](https://proceedings.mlr.press/v313/barber26a.html), examine empirical effectiveness under temporal dependence without restoring arbitrary exchangeability assumptions.

## 4. Benchmark matrix

All methods must receive identical forecast vintages, issue times, calibration/evaluation windows, channel definitions, horizon grid, normalization scales, and realized observations.

| Comparator | What it tests | Important limitation |
|---|---|---|
| Point forecast, no interval | Baseline point accuracy and optimizer value without uncertainty | Cannot qualify risk-aware operation |
| Gaussian residual interval | Parametric residual assumption | Tail shape and regime mixture can be wrong |
| Rolling empirical residual quantile | Simple non-parametric baseline | Sensitive to window size and regime history |
| Independent marginal conformal | Per-channel/per-horizon calibration | Does not establish simultaneous block coverage |
| Block-max split conformal (V0.1) | Simultaneous channel × horizon envelope | Exchangeability remains an assumption; global radius can be wide |
| Adaptive Conformal Inference (ACI) | Online adaptation of miscoverage level | Must be evaluated prequentially; marginal long-run behaviour is not a per-regime guarantee |
| Ensemble Conformalized Quantile Regression (EnCQR) | Heteroscedastic, ensemble-based interval sharpness | Must disclose its time-series dependence treatment |
| State-conditioned / local online conformal | Regime- and operating-condition-aware calibration | Can widen intervals; horizon-specific performance can differ |
| Regime-conditioned joint calibration (V0.2 research target) | Conditional path coverage for ramps, peaks and unusual states | Requires preregistered regimes and sufficient independent examples |

Relevant external research already exists and must be treated as competitive work, not ignored:

- **SAFER (2026):** state-adaptive conformal recalibration for wind-power forecasting using hourly Eskom generation from April 2019 to February 2024. The paper reports improved 90% coverage at 3, 6 and 12 hour horizons, mixed results at 1 and 24 hours, and interval widening as a calibration trade-off. [DOI 10.1016/j.egyai.2026.100900](https://doi.org/10.1016/j.egyai.2026.100900).
- **Local online conformal for power-load forecasting (2026):** [DOI 10.1016/j.epsr.2026.113412](https://doi.org/10.1016/j.epsr.2026.113412).
- **Ensemble Conformalized Quantile Regression:** Jensen, Bianchi and Anfinsen, IEEE Transactions on Neural Networks and Learning Systems (2024), [DOI 10.1109/TNNLS.2022.3217694](https://doi.org/10.1109/TNNLS.2022.3217694).
- **Probabilistic and conformal forecasting survey:** ACM Computing Surveys (2025), [DOI 10.1145/3736575](https://doi.org/10.1145/3736575).

V0.1 is a transparent simultaneous-coverage reference method. It is not claimed to outperform adaptive, local or ensemble approaches. A stronger Symtropy result must beat them on matched chronological traces, at matched joint coverage, while keeping intervals sharper and conditional under-coverage controlled.

## 5. Metrics: calibration is not sharpness

Report separately:

1. **Simultaneous block coverage:** fraction of future blocks for which every channel and horizon is inside its interval.
2. **Cell coverage:** fraction of individual channel/horizon cells covered; diagnostic only, not a replacement for joint coverage.
3. **Sharpness:** mean and maximum interval width, normalized by declared channel scales.
4. **Conditional coverage:** by channel, horizon, forecast lead time, season, load band, ramp event, renewable regime, price regime and thermal regime.
5. **Residual diagnostics:** normalized error distribution, tail quantiles, signed bias and dependence/autocorrelation.
6. **Operational relevance:** change in feasible workload flexibility, service completion, energy cost, power shortfall, reserve margin and thermal margin when the intervals are consumed by the optimizer.
7. **Systems cost:** calibration CPU time, memory, data latency, update cadence and abstention rate.

Coverage alone is gameable by arbitrarily wide intervals. Width alone rewards under-coverage. The primary comparison is a coverage-sharpness Pareto frontier, with hard service, reserve and thermal gates kept non-compensable.

## 6. Proposed acceptance targets

These are **research targets**, not achieved results, standards limits or claims about the current implementation.

### Calibration qualification

- Zero look-ahead leakage and zero missing/undeclared channel-horizon cells.
- Evaluate 90%, 95% and 99% nominal envelopes separately.
- Target absolute aggregate coverage error no greater than 2 percentage points when dependence-aware confidence intervals and sample size support that precision.
- Target no more than 5 percentage points under-coverage in any preregistered high-risk regime; report every regime, including failures and insufficient-data cases.
- At matched simultaneous coverage, target at least 10% lower normalized mean interval width than the strongest simple baseline. The stretch objective is to exceed the strongest adaptive/local baseline without worsening conditional coverage.
- Report the fraction of operating periods where calibration must abstain for insufficient history or an unrecognized regime.

As an initial design calculation, 500 effectively independent holdout blocks gives an approximate 95% binomial margin near ±2 percentage points when true coverage is around 95%. This is not a blanket sample-size rule: strong dependence reduces effective information, and the final analysis must preregister a dependence-aware precision or power calculation. The 9/19/99 finite-rank minima above are not substitutes for this design.

### End-to-end energy orchestration comparators

- **Grid flexibility reproduction:** reproduce the 25% power reduction for three hours reported in the 256-GPU field demonstration, on the same or an explicitly matched service/dispatch definition; then seek more flexibility without QoS degradation, reserve depletion, thermal violations or rebound displacement. The paper's result is a comparator, not a guarantee for other facilities. [Nature Energy (2025)](https://doi.org/10.1038/s41560-025-01927-1).
- **Trace flexibility reproduction:** reproduce the up-to-22% load-reduction result on the same Alibaba-trace methodology and then test whether calibrated joint uncertainty plus physical verification increases dispatchable flexibility without violating deadlines. This is a one-week modeled trace result, not universal field capacity. [IJEPES (May 2026)](https://doi.org/10.1016/j.ijepes.2026.111940).
- **DRO comparator:** reproduce the moment-based worst-case CVaR benchmark on its published case before making a better-than-SOTA claim. The published method already couples workload, renewable, cooling and reserve decisions under joint forecast uncertainty. [IJEPES (September 2026)](https://doi.org/10.1016/j.ijepes.2026.112227).
- **Compute productivity:** report joules and cost per completed useful work unit, throughput per kWh, deadline completion, p50/p95 latency and quality of service. Do not substitute GPU utilization for useful completed work.
- **Facility sustainability:** report PUE, WUE, CUE and site boundary definitions alongside useful-work-normalized energy, water and carbon. DOE's [data-centre design best-practices guide (2024)](https://www.energy.gov/sites/default/files/2024-07/best-practice-guide-data-center-design.pdf) recommends a family of facility and workload metrics rather than a single PUE number.
- **Industry context, not a target:** Uptime Institute's [2026 survey](https://intelligence.uptimeinstitute.com/resource/growing-pue-advantage-larger-data-centers) reports average PUE of 1.52 and notes that leading new facilities routinely report 1.3 or lower. Compare only with matching climate, loading, measurement boundary and facility age; do not optimize PUE by sacrificing useful work, water or resilience.

For M&V, baseline adjustment and demonstrated energy improvements, retain declared boundaries and methods under [ISO 50006:2023](https://www.iso.org/standard/79367.html). A calibration pass is not an M&V result.

## 7. Research sequence

1. **V0.1 — simultaneous holdout reference:** this evaluator and its hostile fixtures; establish leakage-resistant, block-level baseline measurements.
2. **V0.2 — regime-conditioned calibration:** preregister high-risk regimes (load ramps, PV ramps, price spikes, hot ambient, cooling saturation) and compute conditional coverage/width, with explicit insufficient-regime-data outcomes.
3. **V0.3 — prequential adaptation:** evaluate ACI, local online conformal and SAFER-like operating-condition adaptation with immutable issue-time receipts, bounded update rules, delayed labels and drift alarms. Keep an untouched terminal test set.
4. **V0.4 — joint scenario calibration:** test scenario path coverage, cross-channel dependence and tail co-occurrence separately from marginal interval calibration. Only then may an ensemble feed #3581's weighted risk evaluator.
5. **Decision benchmark:** run deterministic risk-neutral MILP, sample-average CVaR, moment-based DRO-CVaR, robust-box, MPC and scenario MPC on identical forecast vintages and realized traces. The independent workload, stateful-resource and scenario-risk verifiers remain the adjudicators.
6. **Field qualification:** replay traceable forecasts and measurements, then use an approved shadow-mode trial before any actuation. M&V compares realized service and energy against a frozen counterfactual baseline.

## 8. Interpretation

A successful V0.1 report means only: the supplied input had valid identities, chronology and a complete grid; a finite split-conformal multiplier was computed; and the stated empirical holdout coverage was measured against that multiplier.

It does **not** mean that forecasts are correct, that arbitrary future conditions are covered, that a joint probability distribution is calibrated, that a DRO guarantee exists, that an optimizer is superior, that a facility is efficient, or that physical/grid operation is qualified.
