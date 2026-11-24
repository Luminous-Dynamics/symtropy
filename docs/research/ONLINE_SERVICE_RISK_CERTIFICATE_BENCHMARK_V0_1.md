# Online Service Risk Certificate and Temporal Shift Monitor v0.1

**Programme:** [ONLINE-SERVICE-RISK-CERTIFICATE-001 #7289](https://github.com/Luminous-Dynamics/symtropy/issues/7289)  
**Parent:** [SERVICE-PREDICTION-CALIBRATION-001 #7240](https://github.com/Luminous-Dynamics/symtropy/issues/7240)  
**Branch:** research/service-online-risk-certificate-v0-1  
**Research cutoff:** 24 November 2026  
**Status:** bounded Rust monitor, strict schema, asynchronous synthetic fixture and CLI; exact-head workflow verification pending.

## 1. Why static calibration is not enough

The service-prediction calibration audit (#7241) binds forecasts to exact site, asset, configuration, model, metric and measurement boundary. It uses chronological calibration/holdout cohorts and refuses to let synthetic evidence create field qualification. The decision-impact evaluator (#7265) adds threshold-level consequences, while the federated reducer (#7268) shares compatible summaries without overriding local support.

A remaining gap is operational time: after deployment, outcomes arrive late and out of order relative to later forecast issue times. A static holdout report can say how a model behaved in the past, but the live service must know exactly which observations were available when a new forecast was issued.

The v0.1 monitor therefore changes the calibration unit from a frozen split to a **prequential evidence sequence**:
- each forecast has an issue timestamp and target interval;
- each outcome has a later evidence-availability timestamp;
- a forecast may consult only outcomes available strictly before its own issue time;
- the new outcome updates subsequent forecasts, never its own decision or an earlier decision;
- exact asset and configuration changes create a new history, not an inherited confidence claim.

This is not a forecast model, scheduler or second hard-safety gate. It reports whether the historical evidence supports continued use of a forecasting signal. It cannot override resource reserves, recovery, commitment leases, journal reconciliation, metering or network gates.

## 2. Paradigm shift: evidence applicability is a time-varying state

A forecast should not carry a timeless calibration badge. Its evidence has an applicability envelope: exact model/version, site/configuration, physical asset/configuration, operating stratum, metric, measurement boundary, observation cutoff and sample history.

A new pack configuration, BMS firmware, charger, site topology, stratum taxonomy or model version starts with no inherited online support. Similar nameplate values or labels are not enough to transfer a certificate.

The monitor exposes three separate things:
1. **Historical risk bound:** an anytime upper bound for the average predictable interval-miscoverage rate over the observed sequence.
2. **Drift alarm:** a deterministic one-sided CUSUM that signals sustained excess misses against a configured risk budget.
3. **Applicability status:** insufficient history, historical risk above budget, drift alarm latched, or historical risk within the configured budget.

A historical-risk-within-budget result is not a guarantee for the next outcome. It means only that the declared historical sequence passed the declared monitor under its assumptions. Reserve and operational gates remain independent.

## 3. Bounded martingale upper confidence sequence

For one exact group, let L_i in {0,1} indicate whether the later observed value fell outside the forecast interval. Define the predictable conditional miss probability mu_i = E[L_i | F_(i-1)], where F_(i-1) is the pre-outcome information filtration.

The v0.1 Rust monitor uses a union-bound allocation:

    alpha_n = alpha * 6 / (pi^2 * n^2)

and a one-sided Hoeffding-Azuma radius:

    r_n = sqrt(log(1 / alpha_n) / (2 * n))

The reported integer-permille upper bound is:

    U_n = min(1000, ceil(1000 * (mean(L_1..L_n) + r_n)))

Because sum over n of 6/(pi^2*n^2) equals 1, the union-bound error expenditure over all observed prefix lengths is at most alpha. Under bounded martingale-difference assumptions, this is a time-uniform upper confidence bound for the **average predictable historical miscoverage** across the observed prequential sequence.

Important limits:
- It is not a bound on the next sample's conditional miscoverage probability.
- It does not guarantee arbitrary future shift, exchangeability, covariate overlap or correct specification.
- It does not prove a battery, charger, aircraft, grid or service is safe.
- It is not a reproduction of the full OMPB PAC-Bayes theorem. It is a smaller transparent martingale monitor and benchmark baseline.
- Wilson intervals from the static v0.1 audit remain IID-reference diagnostics; this time-uniform bound has a distinct martingale assumption and interpretation.

The monitor requires alpha, a maximum historical miscoverage budget and a minimum number of already-available observations. If the minimum history is not met, status remains insufficient-history.

## 4. Separate CUSUM drift alarm

A confidence sequence on the cumulative historical average can respond slowly to an abrupt recent shift. A second deterministic channel therefore maintains a one-sided CUSUM in permille-observation units.

For configured maximum miscoverage q and slack s, update:

    C_t = max(0, C_(t-1) + 1000*L_t - q - s)

When C_t reaches the configured alarm threshold, the alarm latches for the remainder of the evidence epoch. A new evidence epoch requires an explicit new model/configuration/epoch identifier and a separately qualified history; v0.1 does not silently reset a drift alarm.

CUSUM is a detection heuristic, not a probability guarantee. Its false-alarm rate and detection delay must be measured on matched stationary and shifted traces. The confidence sequence and CUSUM are never blended into one scalar score.

## 5. Predict-before-update and delayed outcome semantics

Records are grouped by exact site, site configuration, asset, asset configuration, stratum, model/version, metric, measurement boundary and nominal interval coverage. Within each group:
1. Sort forecasts by strictly increasing issue time.
2. Before each forecast, apply only pending outcomes with outcome-availability time strictly earlier than forecast issue time.
3. Break equal outcome-availability timestamps by stable sample ID; there is no forecast between simultaneous outcome arrivals in the replay.
4. Emit the prequential assessment from the history available before the current forecast.
5. Enqueue the current forecast's later interval-miss outcome.
6. At the audit cutoff, consume all remaining outcomes whose evidence availability is at or before that cutoff for the final retrospective report.

The pending queue is ordered by outcome availability, not forecast issue order. This is necessary because a later-issued forecast can finish and be observed before an older, longer-horizon forecast.

Forecast evidence must be observed no later than forecast issue; outcome evidence must be observed at its declared availability timestamp and no earlier than target end. The exact asset/configuration must match both evidence envelopes. Unknown evidence, duplicate sample/evidence IDs, invalid boundaries, stale/future chronology and malformed horizons are rejected.

## 6. Frozen synthetic known answer

The fixture contains five observations for one exact site/ESS/configuration/model/stratum/metric cohort:
- two interval misses arrive before the fourth forecast is issued;
- the third observation arrives after its own forecast and before the fourth;
- two later forecasts are issued while the CUSUM alarm is already latched.

Expected report:

| Output | Expected |
|---|---:|
| Observations | 5 |
| Exact groups | 1 |
| Interval misses | 2 |
| Final empirical miscoverage | 400‰ |
| Shift-alarmed groups | 1 |
| Risk-above-budget groups | 0 (drift status has precedence) |
| Insufficient-history groups | 0 |
| Prior-observation counts at forecast issue | [0, 0, 0, 2, 3] |
| First three prequential statuses | insufficient_history |
| Fourth/fifth prequential statuses | drift_alarm_latched |
| Model update / field qualification / operational authority | false |

All evidence is synthetic. This fixture checks chronology, arithmetic, report determinism and claim ceilings; it is not a battery or mobility performance result.

## 7. Current SOTA and industry comparison

### 7.1 OMPB — the direct temporal comparator

Huang, Ma and Michailidis, *Model-Agnostic Online Certificate-Driven Calibration for Time Series Forecasting Under Distribution Shift*, UAI 2026 / PMLR 337:2244–2273, develop an online martingale PAC-Bayes certificate using predictable variation, a source-risk anchor, posterior-shift penalty, time-adaptive mismatch and a gated residual Bayesian head. The method follows predict-then-update: outcomes are used only after they are revealed. It targets temporal dependence and covariate/concept shift rather than pretending observations are IID.

Paper: https://proceedings.mlr.press/v337/huang26b.html  
Official implementation: https://github.com/chenfeng-huang/OMPB-UAI-2026

The v0.1 monitor deliberately does not claim OMPB's PAC-Bayes guarantee or reproduce its residual-learning component. The next tournament should run the official OMPB implementation on its native benchmark first, then adapt a frozen forecasting backbone to mobility data with identical chronological cutoffs. The monitor contributes exact physical-asset grouping, evidence timing, hard-gate precedence and an auditable no-learning baseline.

### 7.2 Federated joint shift

Shi et al., *Valid and Efficient Uncertainty Quantification for Federated Joint Shift*, UAI 2026 / PMLR 337:6232–6260, propose FCPJS: a global sketch of nonconformity scores combined with client-local importance-weighted calibration. They report marginal coverage up to a finite-sample/sketch approximation term and a 12.64% predictive-efficiency improvement over the strongest baseline on their four heterogeneous benchmarks.

Paper: https://proceedings.mlr.press/v337/shi26a.html

FCPJS is the stronger federated comparator; the fixed-bin reducer in #7268 is only a deterministic baseline. Online local evidence must remain separate from fleet aggregation. The fleet summary must never override a local drift alarm or unsupported configuration.

### 7.3 Industrial operational practice

The current benchmark should compare against practical deployment baselines, not only academic forecasters:
- fixed safety margin with no online update;
- rolling-window empirical coverage and interval widening;
- exponentially weighted residual quantiles;
- Page-Hinkley/CUSUM monitoring;
- adaptive conformal inference;
- online martingale/PAC-Bayes calibration (OMPB);
- each method behind identical reserve, recovery, lease, PCC and network gates.

The objective is not the smallest forecast error alone. It is the best risk/coverage/service frontier while maintaining zero hard-gate violations.

## 8. Benchmark tournament and targets

All methods must use the same frozen forecast issuance times, target horizons, measurement boundaries, delayed-outcome schedule, pack/site groups and shock scenarios.

| Benchmark | Research target |
|---|---|
| Predict-before-update replay | 100% agreement with hand-constructed asynchronous schedule |
| Future outcome leakage | 0 |
| Cross-asset/config transfer | 0 |
| Duplicate sample/evidence acceptance | 0 |
| Order-invariant report/digest | 100% |
| Frozen repeated-shift detection | 100% alarm by preregistered detection delay |
| Stationary-null false alarms | Report empirical rate with block/cluster uncertainty; target ≤5% per epoch |
| Step-shift detection delay | P50/P95 reported across shift sizes; initial target ≤10 revealed outcomes for a 2× baseline miss rate |
| Risk-bound coverage | Empirical martingale-certificate coverage across preregistered bounded sequences; publish assumptions and violations |
| Utility | Pareto improvement in false-safe admissions, abstention, service success and recovery under identical hard gates |
| Runtime | p50/p95/p99 at 1k, 5k and 50k records; initial target p99 <10 ms for 5k bounded records |

The targets are research goals, not demonstrated performance. Zero violations in a finite synthetic fixture is not a real-world safety probability.

### 8.1 Adversarial and statistical negative controls

- Outcome arrives after a forecast that attempts to consume it.
- Outcome arrives before target end.
- Forecast evidence is observed after issue time.
- Older long-horizon outcome arrives after a later short-horizon outcome.
- New asset configuration attempts to inherit prior calibration.
- Model version or measurement boundary changes within an ostensibly shared group.
- Repeated sample ID or evidence reference.
- Equal forecast issue times in one exact group.
- Stale/future evidence, unknown evidence class and audit-cutoff mismatch.
- Two misses create a CUSUM alarm; a later run of covered intervals does not silently erase the latched alarm.
- Stationary Bernoulli/martingale sequences test false alarms; step, ramp, seasonal and adversarially clustered miss sequences test detection delay.
- Missing outcomes and delayed reporting test the difference between forecast issue time and outcome availability.

## 9. Next steps

1. Reproduce the official OMPB method and native benchmark; document environment and assumptions.
2. Add a rolling-window and adaptive-conformal baseline to the same trace harness.
3. Add block/cluster-aware false-alarm analysis for depot, pack, charger and weather event dependence.
4. Connect the prequential applicability status as an input to #7265 decision-impact replay, without granting it authority to bypass physical gates.
5. Connect the local certificate to #7268 only as a site-local status; the reducer may report aggregate statistics but cannot erase a local alarm.
6. Introduce an explicit model/configuration evidence epoch and a separately authorized reset procedure.
7. Run measured depot trials with independently timestamped forecast/outcome capture before any field qualification claim.

## 10. Claim ceiling

The v0.1 result is a bounded historical-risk monitor. It does not train a model, certify next-step conditional risk, guarantee arbitrary distribution-shift robustness, qualify hardware, establish grid interconnection, authorize market dispatch or replace operator/protection authority. A forecast marked within historical budget remains subject to all existing hard reserve, recovery, commitment, meter, PCC and network gates.
