# Forest observation cadence and assimilation scheduling under latency v0.1

**Issue:** [#5492 — FOREST-OBSERVATION-CADENCE-001](https://github.com/Luminous-Dynamics/symtropy/issues/5492)  
**Composes with:** [#2171 active sensing](https://github.com/Luminous-Dynamics/symtropy/issues/2171), [#5223 forest decision value](https://github.com/Luminous-Dynamics/symtropy/issues/5223), [#5464 parameter measurement design](https://github.com/Luminous-Dynamics/symtropy/issues/5464), [#5487 as-of integrity](https://github.com/Luminous-Dynamics/symtropy/issues/5487), [#5485 usefulness limits](https://github.com/Luminous-Dynamics/symtropy/issues/5485)  
**Date:** 16 November 2026  
**Status:** synthetic scalar state-space benchmark; no real forest cadence optimum

## 1. Paradigm shift: optimize information freshness, not sample count

An observation schedule should be evaluated by when each measurement becomes available relative to the process and the decision deadline—not by the nominal sensor collection cadence alone. A sensor measuring daily with seven days of processing latency provides no more recent information at a seven-day decision cutoff than a single week-old sample. A laboratory assay with better analytical precision may be nearly irrelevant to a decision that occurs before the result arrives.

A 2024 reservoir forecasting experiment tested daily, weekly, fortnightly and monthly data assimilation. Daily updates were most skillful for 1–7-day forecasts; weekly updates often outperformed daily updates for 8–35-day horizons, with results varying by season and depth. The paper also notes that latency and over-frequent parameter tuning can reduce skill. This is a methodological comparator, not a tropical-forest result: https://doi.org/10.1002/ecs2.4752.

Dietze et al. define data latency as collection-to-availability time and note that laboratory analyses and QA/QC can increase it: https://pmc.ncbi.nlm.nih.gov/articles/PMC5816139/. NEON publishes distinct acquisition and publication schedules by forecast target, illustrating the operational importance of latency: https://projects.ecoforecast.org/neon4cast-docs/CronTiming.html.

## 2. Synthetic process and observation contract

For known-truth development, use a stationary scalar AR(1) process:

`x[t+1] = phi*x[t] + eta[t]`

with stationary variance one and innovation variance `1 - phi^2`. Each observation stream declares:
- sampling cadence in days;
- collection-to-availability latency;
- observation-error variance;
- field-effort cost per sample;
- ecological-disturbance cost per sample.

At a decision deadline `D`, a sample at time `t` is eligible only if:

`t + latency <= D`

Eligible measurements are assimilated in collection/event-time order with a scalar Kalman variance update and propagated to the decision deadline. There are no random draws; every profile and portfolio is deterministic given the declared Gaussian model.

The benchmark compares six schedules: daily sensor with seven-day latency; every-third-day remote sensing with two-day latency; weekly field data with one-day latency; fortnightly root cores with two-day latency; monthly high-precision laboratory data with ten-day latency; and weekly lab data with ten-day latency.

Two process contexts use `phi=0.98` (persistent/stable) and `phi=0.8` (volatile/post-disturbance), at decision deadlines 7, 30 and 90 days. These coefficients and all sensor characteristics are synthetic.

## 3. Budgeted portfolio result

For every context/deadline, the validator enumerates all schedules with at least one usable measurement under two separate limits:
- field effort budget: 4.2 synthetic effort units;
- ecological disturbance budget: 0.8 synthetic disturbance units.

It reports the best posterior variance and the cost–uncertainty Pareto frontier, while retaining measurement count, latest usable observation age and disturbance cost. A profile with no observations available by the decision deadline cannot be selected as a zero-cost, zero-information portfolio.

The best schedule is context- and deadline-dependent. It is not the policy with the highest nominal frequency by default; only actually available observations affect the posterior at decision time. The fixture records all profile summaries, feasible portfolio counts, best portfolios and Pareto frontiers.

## 4. How this differs from generic EVSI

This kernel is a transparent process/measurement-timing adapter. Posterior variance is an information-state diagnostic. It is not the value of a conservation decision. The actual management action, asymmetric loss, access, field logistics and EVSI must be evaluated by #5223. Generic active-sensing policy semantics remain with #2171.

## 5. Empirical tournament protocol

For real forests, build cadence × latency × horizon surfaces separately for variables such as biomass, mortality, canopy condition, root turnover, soil moisture and nutrient flux. Preserve source-native collection, release, QA, rights and ingestion timestamps from #5487. Pair each schedule on identical forecast origins, process states, block/site splits and forecast-driver versions. Include sensor downtime, missingness, calibration drift, field maintenance, access costs and sampling disturbance.

A useful empirical design should compare:
- daily or continuous sensing;
- event-triggered / threshold sampling;
- weekly or fortnightly field campaigns;
- monthly laboratory assays;
- multi-stream fusion that combines rapid low-precision observations with delayed high-precision measurements;
- no-update and fixed-rotation baselines.

Report predictive scores and decision loss separately. The minimum useful cadence must be estimated by variable, process regime and decision horizon, with held-out sites and seasons.

## 6. Claim ceiling

Synthetic PASS verifies only the declared scalar-Gaussian variance and portfolio arithmetic. It does not establish AFEX data latency, actual sensor performance, an ecological process timescale, field cost, disturbance impact, monitoring optimum or forecast superiority.
