# Maritime Censoring Robustness — Industry Comparator Register

**Research snapshot:** 21 November 2026.

## CPPI data and measurement methodology

The World Bank / S&P Global CPPI 2025 methodology note (10 June 2026) reports:
- call-level operational timestamps supplied by ten of the largest liner shipping companies, representing approximately 75–80% of global container fleet capacity;
- AIS-derived movement records checked against geofences;
- a reported 95% match rate for verified port-call records;
- six key events: anchorage/pilot station arrival, movement to berthing place, cargo start, cargo end, berth departure and port-limit exit;
- vessel capacity, call size and crane deployment information;
- rigorous removal of incomplete, inconsistent and duplicated records;
- ship-size and call-size stratification in the index construction.

Sources:
- https://thedocs.worldbank.org/en/doc/aac122f6df85534428d66a7b9af4b7f6-0400012026/original/CPPI-Methodology-Note.pdf
- https://www.worldbank.org/en/topic/transport/publication/cppi

The CPPI's 95% match rate is a programme-specific reported statistic, not a universal AIS completeness target. A local station's performance must be assessed against independent port-call references, with unmatched, unobserved, duplicated, stale and disputed calls reported by regime.

## 2026 low-cost edge AIS field benchmark

Ding, Indriyanto, Hidayat & Lestari, *Transportation Research Interdisciplinary Perspectives* 40 (November 2026), 102342:
- DOI: https://doi.org/10.1016/j.trip.2026.102342
- Title: *An edge-based AIS finite-state machine framework for real-time port ship service performance monitoring*.
- Uses a sub-USD-300 RTL-SDR / Raspberry Pi edge receiver and a five-state dwell-confirmed finite-state machine over configured port, berth and anchorage polygons.
- Reports agreement with crew-reported navigation status `κ=0.96` over 917 calls, 26 of 27 berthings detected against terminal schedules, and median departure-time error of 0.4 hours.
- Reports approximately ±10% variation in KPIs when dwell and gap parameters change.

This is a directly relevant open-access field comparator. It is not an independent gold standard for all event types: its validation set includes crew status and terminal schedule references, and the berth schedule denominator is only 27 calls. It also reports about ±10% KPI sensitivity to dwell/gap parameter changes, so parameter uncertainty belongs in the benchmark. We should reproduce its five-state FSM/profile if source code/data can be obtained, then challenge it with known missing-message gaps, duplicate/replayed AIS, polygon boundary transitions, dwell/gap sensitivity, receiver outages and independently adjudicated event labels.

Paper: https://www.sciencedirect.com/science/article/pii/S2590198226005075

## Port time variability research

Chhetri et al., *European Transport Research Review* 18, 107 (13 November 2026), DOI https://doi.org/10.1186/s12544-026-00862-w:
- Proposes coefficient of variation of port-call time as a scale-free reliability metric.
- Analyses 401 ports across 119 countries using AIS, World Port Index and country-level covariates.
- Distinguishes congestion (system state), delay (outcome) and variability (reliability).
- Identifies limitations from port-level aggregation, country-level covariates, timestamp relay quality and historical data scope.

This is a comparator for **predictability**, not a censoring correction. Extend the maritime scoreboard to report mean, standard deviation/CV, P50/P90/P95, queue area and service reliability together. CV can be unstable when mean durations are very small and says little about the shape of extreme tails; never replace tail metrics with CV alone.

Paper: https://link.springer.com/article/10.1186/s12544-026-00862-w

## Censoring method comparators

- Zabor & Eaton (2026), *Pharmaceutical Statistics*, DOI https://doi.org/10.1002/pst.70138: tutorial/review of inverse probability of censoring weights for single and competing events under dependent censoring. Use as a method reference, not a maritime model.
- Al-Zalzalah (13 November 2026), *AppliedMath* 6(11), 196, DOI https://doi.org/10.3390/appliedmath6110196: shared-gamma dependence model with partial-identification/sensitivity analyses for competing risks. Its reported profile-grid ranges are model-specific and are not certified sharp bounds.
- Alberge et al. (AISTATS 2025 / PMLR 258), *Survival Models: Proper Scoring Rule and Stochastic Optimization with Competing Risks*: direct competing-risk proper-score baseline. https://proceedings.mlr.press/v258/alberge25a.html

Internal reusable work:
- WPT Survival #3763 / draft PR #3791: multicause IPCW Brier, hazard log score and competing-risk forecast integrity.
- RUL #4028 / draft PR #4035: calibration-partition censoring survival, positivity and grouped leakage checks.
- WPT calibration #3692: dependence-aware uncertainty / cluster split discipline.

## Comparative targets

- **AIS observability:** compare edge station versus reference receiver/port schedule across coverage, event recall/precision, false states, time error, latency and message-gap robustness.
- **Source delay:** report publication delay distributions by provider, terminal, event cause and congestion regime; no global independence assumption.
- **Censoring support:** publish `G(t|X)`, minimum support, maximum weight, ESS and positivity failures for every horizon/regime.
- **CIF accuracy:** compare to full truth in simulation; for real data, use only identified estimands or explicit sensitivity intervals.
- **Reliability:** combine mean, CV, median, p90/p95, queue-area exposure and delivery-window failure.
- **Reproducibility:** pinned polygons, state-machine version, dwell/gap parameters, AIS message archive, raw byte hashes and exact reference adjudication.

Do not claim SOTA from a 95% CPPI match rate or the edge AIS study's single-port results. Reproduce with matched event definitions and independent validation.
