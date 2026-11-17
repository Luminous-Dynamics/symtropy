# WDN cross-population benchmark protocol v0.1

**Tracking issue:** #4979 WDN-DATASET-CROSSWALK-001  
**Parent benchmark:** #4963 PRESSURIZED-WATER-LEAK-DIAGNOSTICS-BENCH-001  
**Date:** 17 November 2026  
**Status:** metadata-only protocol; no raw dataset files admitted.

## 1. Why this increment exists

The current literature makes clear that "leak detection accuracy" is not a single comparable quantity. A 30-second laboratory hydrophone trace, an hourly utility SCADA record, a 5-minute competition network and an EPANET simulated scenario have different sampling, labels, topology and observability.

The 2026 EPANET/WNTR Net3 paper reports 47,000+ generated scenarios and >95% detection accuracy, while also noting localization remains harder. This is a synthetic reference that must be reproduced with its exact split and scorer before it can be used as a baseline ([Ranganathan et al., May 2026](https://doi.org/10.1016/j.jwpe.2026.110114)).

The Slovak utility dataset is a different evidence population: one year of hourly data from a single distribution zone, 8,737 records and 18 variables; its binary `fault_d7` label is constructed with a ±7-day temporal window. It cannot support minute-scale onset or acoustic localization ([Babela et al., Scientific Data, 17 April 2026](https://doi.org/10.1038/s41597-026-07203-5)).

The Aghashahi LeakTestbed is a controlled physical lab set: 280 30-second signals from a 47 m, 152.4 mm PVC test network, using two accelerometers, two hydrophones and two dynamic-pressure sensors; four leak mechanisms plus no-leak are represented. This is physical signal evidence, but not municipal-network validation ([Data in Brief](https://doi.org/10.1016/j.dib.2023.109148); [WaterBenchmarkHub](https://waterfutures.github.io/WaterBenchmarkHub/benchmarks/LeakTestbed.html)).

Two 2026 systematic reviews report substantial fragmentation in datasets, metrics and evaluation practice (90-study review: https://doi.org/10.3390/w18182267; D-A-T review: https://doi.org/10.3390/w18212797). These support the need for strict population-specific protocols, not a claim that one model is universally best.

## 2. Evidence classes and population-specific claim ceilings

Eight populations are encoded in `wdn-cross-population-protocol.v0.1.json`:

| Population | Evidence | Allowed use | Cannot establish |
|---|---|---|---|
| DiTEC-WDN | Synthetic normal hydraulics | P0 normal-state modelling | Leak-positive performance without separately designed labels |
| LeakDB | Synthetic leak cases | P1 synthetic T1/T2 reproduction after rights/artifact admission | Lab or field performance |
| Aghashahi LeakTestbed | Controlled physical laboratory signals | P2 sensor/modality and event classification | Full municipal-network generalization |
| Slovak SCADA | Real, one-zone, hourly utility telemetry | Coarse P3 anomaly detection | Minute-scale detection, acoustic signals or pipe localization from `fault_d7` |
| BattLeDIM/L-Town | Competition simulation/SCADA-style series | Official 2018→2019 benchmark reproduction | Live utility field validation |
| Net3 47k | Paper-reported EPANET/WNTR synthetic benchmark | Exact paper reproduction after artifact recovery | Reproduction from an abstract |
| Pamplona DAS | Controlled field valve events over dark fibre | P2 controlled-field DAS study after raw data and rights admission | Natural leak events or citywide availability |
| WDSEventDB | Sensor testbed events | Event diagnosis after file audit | Utility-scale performance or inferred equivalence to LeakTestbed |

No population is yet file-admitted. The JSON deliberately states no acquired bytes, SHA-256 archives or split manifests.

## 3. Diagnostic task contract

### T1 — Detection

Report PR-AUC, recall at a pre-registered false-alarm budget, precision, false alarms per asset-time and calibration/Brier score. Detection delay is reported only when event-onset precision supports it. Overall accuracy alone is not a primary ranking metric.

### T2 — Localization

Report Top-1/Top-5 only when the candidate set and location ground truth are defined, plus P50/P95 distance error, distance-hit rate, candidate-set size, ambiguity-set coverage, abstention and risk-coverage. Zone-level predictions do not count as pipe-level predictions.

### T3 — Characterization

Report MAE/bias in native leak-flow units, P50/P95 absolute error and prediction-interval coverage/width. Do not infer flow magnitude from a leak-type class or a simulated orifice setting without independent ground truth.

### Physical identifiability

Before measuring conditional model skill, calculate the fraction of fault pairs distinguishable at the deployed sensor layout and under declared single/common-cause sensor failures. Report diagnosability survival curves as channels and dependency domains fail. If the fault is not physically identifiable, return an ambiguity set or abstention.

## 4. Split rules

- **Synthetic scenario corpora:** hold out complete generated scenarios, network/topology IDs and simulation seeds. Reproduce the source's original split before proposing an alternative.
- **Physical testbeds:** group by complete physical experiment and configuration. All windows from one 30-second recording/event stay in one partition. Hold out leak mechanisms, topology or sensor location for transfer only when the raw labels support those factors.
- **Hourly Slovak SCADA:** use blocked chronological partitions. Because `fault_d7` labels use ±7 days, purge at least the full label window plus a preregistered buffer around boundaries; all hours affected by one event stay in one partition. Do not claim fine onset/lead time.
- **BattLeDIM:** preserve the official 2018 historical/training and 2019 evaluation split and official scoring/economic rules for headline reproduction.
- **Pamplona DAS:** hold out complete controlled valve events, route segments and environmental periods; a one-route study supports only within-route transfer.
- **WDSEventDB:** group complete events and testbed configurations; keep leak, cyberattack and sensor failure distinct.
- **Pooled metrics:** prohibited. Train→test transfer is reported as a matrix, with each target population scored independently and source/target domain shift declared.

Preprocessing, scaling, feature selection, calibration and threshold selection are fitted on training data only.

## 5. Baseline ladder

Reproduce in this order:
1. persistence / last-valid-state;
2. fixed threshold and CUSUM;
3. water balance / minimum-night-flow where the zone boundary and metrology permit it;
4. hydraulic residual / EPANET-WNTR;
5. Random Forest / XGBoost;
6. CNN / temporal model;
7. GNN / spatiotemporal graph;
8. multimodal fusion only after separate modality baselines and dependency/common-cause tests.

Fusion earns credit only for incremental value at the same sensor count, sensor lifecycle cost, sample rate, missingness and physical identifiability.

## 6. Proposed targets — not achieved results or standards

- T1: investigate at least 95% recall at a preregistered false-alarm budget **per eligible population**, not pooled.
- T2: investigate at least 25% lower P95 localization error than the strongest reproduced baseline at equal sensor budget and equal identifiability coverage.
- T3: investigate at least 90% empirical prediction-interval coverage on a separate population with independently measured leak-flow ground truth.
- Sensor efficiency: publish Pareto curves over sensor count, lifecycle cost, maintenance burden, observability and false-alarm burden; no hidden weighted winner.

These are stretch hypotheses. They are not SOTA claims, achieved performance or universal acceptance thresholds.

## 7. Industry comparison and falsification

The most important comparison is not one overall accuracy number. It is whether the system reduces real inspection and repair burden at a fair sensor budget without hiding unobservable faults or increasing missed severe leaks.

A fair comparison freezes source release, event definition, sensor principles and count, sample rate, missingness, network topology and split. Include simple operational baselines and the strongest published/reproduced baseline. Report all target populations independently.

A claim of superiority fails when:
- an event label is too coarse for the claimed warning lead;
- a pipe prediction is made without pipe-level ground truth;
- any preprocessing or windowing leaks event information across partitions;
- the model exceeds the declared sensor count or uses unavailable channels;
- common-cause failure is treated as independent redundancy;
- abstentions/unobservable cases are silently dropped from the denominator.

## 8. Next execution order

1. Preserve this machine-readable protocol and the existing rights-aware source crosswalk.
2. Recover exact artifact versions and terms for the selected benchmark populations.
3. Admit one dataset at a time with raw archive digest, file inventory, parser revision and label audit.
4. Publish a group/event-level split manifest before model training.
5. Reproduce the simplest baseline and the paper's official scorer.
6. Only then evaluate Symthaea models and cross-population transfer.
7. Require independent physical or utility evidence before field performance, repair savings or water-loss reduction is claimed.

## 9. Nonclaims

This is a metadata-only protocol. No raw data have been acquired or hashed. No leak detector, physical layout, utility model, field prediction, pipe location, water saved, repair outcome or SOTA improvement is claimed.
