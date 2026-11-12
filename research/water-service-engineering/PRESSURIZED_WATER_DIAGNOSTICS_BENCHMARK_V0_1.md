# PRESSURIZED-WATER-LEAK-DIAGNOSTICS-BENCH-001 — Benchmark Protocol v0.1

**Tracking issue:** [Symtropy #4963](https://github.com/Luminous-Dynamics/symtropy/issues/4963)  
**Status:** research contract; no detector or field system qualified  
**Scope:** household plumbing, community/district-metered areas and municipal pressurized distribution.

## 1. Research question and paradigm

Can we turn an anomaly into a useful, independently reviewable chain:

`detect -> localize -> estimate leak magnitude/uncertainty -> identify observability limits -> prioritize inspection -> independently verify repair -> measure service outcome`

A pressure anomaly is not automatically a leak. A detected leak is not necessarily localized. A model recommendation is not permission to close a valve, excavate or alter service.

Measure task-level usefulness rather than frame-level classification or a pooled accuracy number. The target is a verified leak event with a useful location, calibrated uncertainty, manageable false-alarm burden and a repair outcome supported by independent evidence.

## 2. Existing Luminous ownership

Compose without duplicating:
- Symtropy #4397/#4398 for distribution hydraulics, pressure/leakage models, OWA-EPANET comparisons and solver feature qualification.
- Symtropy #4925/#4953 for localization and acoustic/visual inspection methods.
- Symtropy #4905 for gravity-sewer robot inspection; this benchmark is for pressurized water pipes and household distribution.
- Symtropy #4959 for pump-system duty and delivered service.
- Mycelix #6034/#6057/#6025/#6029 for instrument passports, calibration, condition and measurand semantics.
- Mycelix #5431 for provenance-first Johannesburg service evidence.

No second hydraulic solver, asset registry, generic telemetry fabric, instrument passport, event ledger or valve-control authority is introduced.

## 3. State-of-the-art comparison snapshot (2026)

| Comparator | Population / method | Reported evidence | Important limit |
|---|---|---|---|
| Ranganathan et al., May 2026 | 47,000+ EPANET/WNTR Net3 scenarios; ML, DL, GNN, Transformer | Authors report >95% leak detection accuracy; localization remains harder | Synthetic network scenarios; not a field-deployment score |
| Babela et al., 17 Apr 2026 | One year hourly SCADA, flow/pressure, power and environment data from one Slovak utility zone with leak records | Open real-utility time series | One zone; hourly sampling; geography and label coverage constrain transfer |
| LeakTestbed | 280 laboratory signals from 47 m, 152.4 mm PVC loop; accelerometers, hydrophones and dynamic pressure | Four seeded leak mechanisms plus no-leak; multiple loop/branch, flow, noise and sensor conditions | Small physical rig; not a full municipal network |
| 2026 systematic review | 86 studies from 2015–2026 | Identifies sensing, validation, localization and comparison gaps | Review evidence is not a single shared leaderboard |

Sources:
- https://doi.org/10.1016/j.jwpe.2026.110114
- https://doi.org/10.1038/s41597-026-07203-5
- https://waterfutures.github.io/WaterBenchmarkHub/benchmarks/LeakTestbed.html
- https://doi.org/10.2166/hydro.2026.069

These datasets must remain separate benchmark populations. Do not pool their accuracy or treat the simulated 47k scenarios as real-world field evidence.

## 4. Scale-specific sensing and service

### Household / building
Smart-meter intervals, fixture/branch flow, accessible pressure and acoustic/vibration channels. Separate continuous low-flow leaks from appliance events, occupancy shifts and legitimate use. Metric denominators include false call-outs per household-month and confirmed water loss.

### Community / district-metered area
Zone inlet/outlet flow, pressure logging, minimum-night-flow, step tests where approved, pressure-transient/acoustic correlation, demand and boundary-valve state. Optimize sensor placement for observability and practical repair localization.

### Municipal / trunk and distribution
Hydraulic residuals against a qualified EPANET/WNTR/OWA reference, multi-DMA and multi-sensor evidence, segment localization, leak magnitude and repair/outcome follow-through. Preserve material, age, pressure, elevation and demand context where known.

Candidate methods:
- minimum-night-flow and conventional water audits / step tests;
- threshold / change-point alarms on flow and pressure;
- hydraulic residuals and state estimation;
- acoustic correlators, hydrophones, accelerometers and transient methods;
- classical ML and unsupervised anomaly detection;
- CNN/LSTM/Transformer/GNN where data geometry justifies them;
- physics-guided multimodal residual fusion;
- active sensor placement / sampling by observability and value of information.

## 5. T0 / T1 / T2 / T3 evidence ladder

- **T0:** analytic/synthetic no-leak, seeded-leak, demand/valve/pump confounder, sensor failure and event-semantics cases. Synthetic only.
- **T1:** reproduce public controlled LeakTestbed baselines when data rights and acquisition permit; split by physical run and leak configuration.
- **T2:** retrospective evaluation against the 2026 real-utility SCADA dataset, subject to data licence and metadata; preserve hourly resolution and single-zone scope.
- **T3:** held-out network/site supervised pilot with independent leak confirmation, operator review and site safety approval.

No T0/T1 result is field accuracy. No model score is a water-loss saving or verified repair.

## 6. Dataset discipline and leakage prevention

Freeze topology, pipe material/diameter, demand, pressure, pump/valve operations, event onset, leak mechanism and independent ground truth. Split train/validation/test by complete physical event and, where possible, by network/DMA, asset, season and operator. Do not randomly split adjacent samples or signal windows from the same leak event across folds.

Compare at a fixed sensor and communications budget. Ablate each modality. Report:
- event precision / recall / F1 / PR-AUC by leak size and type;
- false alarms per household-month, sensor-day or DMA-month;
- detection delay;
- segment accuracy and localization distance P50/P95/max;
- leak-volume estimation bias and interval coverage;
- high-confidence wrong-segment probability and abstention;
- sensor availability, missingness and common-cause faults;
- call-outs per confirmed leak, inspection hours and pipe length searched;
- verified repair location and independently measured post-repair change.

Accuracy alone is misleading when leak prevalence is low. Publish class prevalence and precision-recall curves.

## 7. Proposed research gates (not achieved)

- Reproduce the 2026 >95% synthetic Net3 detection result before claiming an algorithmic advance.
- At equal recall, seek >=25% fewer false alarms per DMA-month on a held-out physical or real-utility profile.
- At equal sensor budget, seek >=25% lower P95 localization error on at least one held-out profile.
- Calibrate leak-magnitude intervals on held-out events where sample size permits.
- Report domain-transfer loss by network, pipe material, pressure regime and season.
- Missing telemetry, unobserved pipe length, stale calibration and sensor common-mode bias never imply leak absence.
- No automatic valve or pump action from model-only alerts; use local interlocks and separate authorization.
- Independently verify every field event counted as a confirmed leak or saved-water outcome.

These are aspirational targets, not predicted performance.

## 8. Safe operational integration

Symthaea may rank inspection candidates and explain evidence with uncertainty. Symtropy supplies qualified hydraulic reference and scenario replay. Sol-Atlas provides spatial network context. Mycelix carries source, adjudication, permission, maintenance and outcome evidence. Physical interventions remain with the operator and independent local protections. The detector must remain useful as a read-only system and fail safely when communications/cloud services are absent.

## 9. Nonclaims

No leak detector, field dataset, municipal outcome, saved-water quantity, localization accuracy or repair workflow is qualified by this document. Literature and dataset performance are comparators only. No model result authorizes isolation, excavation, pump/valve actuation or service restoration.
