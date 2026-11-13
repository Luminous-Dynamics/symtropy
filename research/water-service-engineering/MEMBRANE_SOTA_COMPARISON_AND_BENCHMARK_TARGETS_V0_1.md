# Membrane fouling / observability SOTA comparison and benchmark targets v0.1

**Date:** 13 November 2026  
**Programme:** MEMBRANE-FOULING-QUAL-001 (#4977)  
**Evidence status:** literature-derived comparison; all reported scores below are paper-reported and task-specific. None is a result from this repository.

## Executive conclusion

Do not attempt to beat the field by merely replacing GRU/XGBoost/CNN-GRU with a larger neural network. The current bottleneck is not only prediction accuracy. It is measurement trust, mechanism identifiability, temporal generalization, sample-time quality evidence, causal intervention value and safe fallback.

A credible differentiator is a complete evidence chain:

1. identify whether the observation is a physical-process change or measurement-system fault;
2. estimate process state only where it is observable;
3. identify a mechanism only when evidence is discriminative, otherwise abstain;
4. forecast under chronological and regime holdouts;
5. evaluate interventions as counterfactuals with full lifecycle outcomes;
6. deliver independently reproducible receipts at a fixed source revision.

## Recent comparison set

### 1. PI-STGAT — strongest retrieved process-anomaly comparison

Noh et al., *Anomaly diagnosis in SWRO desalination plants using a physics-informed spatiotemporal graph attention network*, Water Research 308 (2026), Article 126904, DOI [10.1016/j.watres.2026.126904](https://doi.org/10.1016/j.watres.2026.126904). Available online 8 September 2026.

The article reports a 1,000 m³/day SWRO demonstration plant, with energy, flow and conductivity indicators. PI-STGAT combines a hybrid physical/domain and mutual-information graph, lag-aware edges for time-delayed process propagation, and SHAP attribution over sensors and time steps. It reports per-fold R² of 0.946, 0.946 and 0.963, average F1 of 0.953, and 2.7–21.7 percentage-point F1 improvements over an MI-only graph; it also reports 13.4–29.5 percentage-point improvements over five deep-learning comparators.

**What it establishes:** physical process pathways and time lags can improve anomaly localization on that plant-specific task.

**What it does not establish:** membrane fouling mechanism identification, sensor-fault-versus-process-fault discrimination across independent plants, long-horizon fouling warning, causal CIP benefit, or cross-site generalization. Do not claim to outperform its F1 until the task, label definitions, site population, class balance and split protocol are matched.

Primary source: https://pubmed.ncbi.nlm.nih.gov/42753656/

### 2. Drift-aware digital-twin readiness — temporal validation comparator

*Drift-Aware Digital-Twin Readiness for Two-Stage Reverse Osmosis: Plant-Scale Time-Aware Machine Learning and Operating-Regime Validation*, Membranes 16(10), 328 (2026).

The study describes 476 train-day records across 17 operating months and 55 later-period holdout records. Random split results included R² 0.876 / 0.833 for second-stage permeate flow and conductivity, while chronological evaluation fell to 0.042 / 0.118 and the later-period holdout to −1.173 / −0.918. The study argues for traceable canonical data, operating-regime surveillance, explicit uncertainty/residual monitoring and human-supervised advisory operation.

**Benchmark implication:** random splits can overstate future utility dramatically. The programme must make chronological, rolling-origin and later-regime holdouts mandatory. A model that wins only under random row splits does not qualify as operationally superior.

Primary source: https://www.mdpi.com/2077-0375/16/10/328

### 3. Morocco six-year industrial RO SCADA

Elyaakouby, El-Kordy & Tilioua, *Analysis and diagnostic of reverse osmosis membrane fouling using industrial plant data*, Cleaner Water 6 (June 2026), Article 100247, DOI [10.1016/j.clwat.2026.100247](https://doi.org/10.1016/j.clwat.2026.100247).

The paper reports six years of industrial demineralization SCADA, approximately 10% deviations in selected differential-pressure, conductivity and pretreatment indicators, a 15–20-day warning window, and SEM–EDS autopsy evidence consistent with organic fouling. The raw data are available on request; the paper says thresholds require site-specific calibration.

**Benchmark implication:** the 15–20-day warning is an external replication target, not a result this programme can claim. Request raw tags, sampling cadence, event dates, CIP history, membrane train identity, calibration records and the original split/protocol. Reproduce on the same population, then test a truly external chronology/site.

### 4. Mechanism-aligned model review

Yang et al., *Mechanism-aligned machine learning for membrane fouling prediction and process control in pressure-driven water treatment membranes: A review*, DOI [10.1016/j.jwpe.2026.110543](https://doi.org/10.1016/j.jwpe.2026.110543).

The review maps model choice to process/signal structure: GRU for sufficiently dense UF hydraulics, regularized XGBoost for multicollinear NF process inputs, and CNN–GRU for heterogeneous RO signals. It also identifies data scarcity, SCADA noise, interpretability, edge deployment and plant-scale validation as practical constraints.

**Benchmark implication:** keep matched baselines for UF, NF and RO; do not pool metrics or assume one architecture transfers across them.

### 5. Drinking-water membrane monitoring / prediction / control review

Arembage et al., *Integrated Approaches to Membrane Fouling Monitoring, Prediction, and Control in Drinking-Water Systems: A Comparative Review*, Frontiers in Water 8 (2026), DOI [10.3389/frwa.2026.1863523](https://doi.org/10.3389/frwa.2026.1863523).

The review synthesizes 78 peer-reviewed formal/contextual studies from 2015–2024 across MF, UF, NF and RO. Its conclusion is that monitoring and prediction are more developed than deployable control, with comparability, external validation, sensor drift, missingness, actuator constraints, safety logic and automation integration still limiting implementation.

**Benchmark implication:** distinguish prediction maturity from control readiness. Report what is measured, simulated, retrospectively inferred, independently verified and physically tested as separate evidence classes.

## Comparison table

| Dimension | Current retrieved field reference | Programme response |
|---|---|---|
| Plant anomaly classification | PI-STGAT average F1 0.953 on one 1,000 m³/day SWRO task | First match task/labels; then add sensor fault, process shift and multiple-cause negative controls |
| Temporal transfer | Two-stage RO random-split R² 0.876/0.833; chronological R² 0.042/0.118; later holdout negative | Rolling-origin, purged horizon and later-regime holdout are mandatory |
| Fouling warning | Morocco study reports 15–20 days; raw SCADA request-only | Exact external replication only after licensed raw data and labels |
| Model choice | GRU / XGBoost / CNN-GRU by process and signal | Strong simple/mechanistic baselines first; model selection per membrane/process |
| Mechanism diagnosis | Mixed evidence and nonunique symptoms | Mechanism-conditioned estimate with explicit INDETERMINATE/MULTIPLE_CAUSES |
| Sensor trust | Drift and data-quality risks remain central | Sensor-fault F0 with parity residuals, negative controls and abstention |
| Intervention | Control readiness lags prediction maturity | Counterfactual cleaning target-trial, lifecycle cost, quality and safety gates |
| Quality evidence | Lab quality often sparse | Sample-time/location/analyte labels only; no interpolation |

## Programme benchmark ladder

### A. Measurement and diagnosis
- Engineering alarm/SOP thresholds.
- Range, rate-of-change, persistence, missingness and timestamp checks.
- Redundant sensor parity and mass/pressure/flow consistency residuals.
- EWMA/CUSUM and multivariate statistical process control.
- Isolation Forest / robust covariance for anomaly-only detection.
- Regularized XGBoost and GRU baselines.
- Physics-informed residual observers.
- Domain graph + lag-aware graph models with ablations.
- Hybrid models with calibrated uncertainty and abstention.

### B. Fouling estimation / warning
- Mechanistic hydraulic resistance / normalized permeability.
- Persistence and trend/EWMA/CUSUM.
- Linear/AR and XGBoost.
- GRU on sufficiently dense UF sequences.
- CNN-GRU on sufficiently rich RO multichannel sequences.
- Physics-informed state observer + residual learner.
- Cross-site/module external evaluation.

### C. Cleaning value
- Calendar schedule.
- Operator/SOP threshold.
- EWMA/CUSUM.
- Mechanistic threshold.
- ML/hybrid advice with abstention.
- Perfect-information policy only as a retrospective upper bound, never a deployable comparator.

## Proposed targets — not achieved

These are staged research targets, not a claim of current performance:

| Gate | Target |
|---|---|
| Synthetic F0 diagnostic | 0 false fouling classifications in sensor-only and regime-only negative controls |
| Held-out synthetic diagnosis | ≥0.90 macro-F1 with unseen fault-family and regime combinations |
| Unobservable process state | ≥0.90 correct abstention on a labelled unobservable holdout |
| Plant event detection | ≥95% recall only after a sufficiently large audited event population, at a pre-registered false-alert budget |
| False alert burden | ≤1 alert per module-week as an initial target; report plant/site variation |
| Warning lead time | Report full lead-time distribution, not just mean; 15–20 days only on Morocco's original RO population |
| Uncertainty | Nominal 90% intervals achieve close to 90% coverage on untouched temporal/regime holdout |
| Transfer | Performance and calibration reported separately for every held-out site/module/feed regime |
| Cleaning policy | Target ≥10% lifecycle-cost reduction per m³ of conforming permeate at equal quality/integrity constraints, supported by causal identification |

Do not combine the synthetic F0 target, process-anomaly F1 and real fouling-event recall into one headline score.

## What would constitute beating SOTA?

A defensible claim requires:
1. identical benchmark task and event/label definitions;
2. matched train/calibration/test access, feature cutoffs and split rules;
3. complete-site/module or later-period holdouts;
4. matched alarm burden and class prevalence;
5. calibration, abstention and worst-regime reporting;
6. sensor-fault and process-shift negative controls;
7. mechanism attribution only when independently identifiable;
8. reproducible input, oracle and evaluator digests;
9. an independent replication or data-provider confirmation;
10. a separate operational-value result for cleaning decisions.

A single 0.99 F1 on a synthetic set is not superior to a 0.953 plant result. The benchmark and generalization evidence must make the comparison meaningful.

## Current repository evidence

- M0: pointwise RO transport formula checks, five synthetic cases.
- M0B: grouped synthetic multi-pressure through-origin regression, three cases and replicate-level variance; not a reproduction of Armstrong's OSF workbook.
- M2: six synthetic sample-time water-quality contract fixtures; no real Bristol lab data.
- Observability F0: 13 synthetic sensor/process confounder cases with a transparent residual-rule baseline.
- Dataset crosswalk: four records; no raw archive bytes acquired.
- CIP counterfactual: target-trial protocol only; no intervention effect identified.

## References

1. Noh et al. (2026), https://doi.org/10.1016/j.watres.2026.126904
2. Drift-Aware Digital-Twin Readiness (2026), https://www.mdpi.com/2077-0375/16/10/328
3. Elyaakouby et al. (2026), https://doi.org/10.1016/j.clwat.2026.100247
4. Yang et al. (2026), https://doi.org/10.1016/j.jwpe.2026.110543
5. Arembage et al. (2026), https://doi.org/10.3389/frwa.2026.1863523
