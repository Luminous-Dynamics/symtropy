# Service Prediction Calibration and Deployment-Shift Benchmark v0.1

**Programme:** [SUSTAINABLE-MOBILITY-SERVICE-QUALITY-001 #6921](https://github.com/Luminous-Dynamics/symtropy/issues/6921)  
**Capacity/receipt owners:** [#6909](https://github.com/Luminous-Dynamics/symtropy/issues/6909), [#6999](https://github.com/Luminous-Dynamics/symtropy/issues/6999)  
**Calibration issue:** [#7240](https://github.com/Luminous-Dynamics/symtropy/issues/7240)  
**Branch:** \`research/service-prediction-calibration-v0-1\`  
**Research cut-off:** 24 November 2026  
**Status:** bounded Rust evaluator, strict schema, synthetic two-stratum fixture, CLI and workflow integration; exact-head Actions verified.

## 1. The missing reliability contract

The service stack now has:
- a deterministic aircraft/pack/charger/site capacity model (#6909 / #6848);
- event-derived service opportunities (#6999 / #7004);
- a linked request consumer that re-evaluates capacity before admission (#7074);
- a full-horizon offline exact assignment upper bound (#7129);
- a nonanticipative scenario-tree oracle in progress (#7133).

It still needs to answer a different question:

> When the forecast says a particular pack will have 1,200 Wh available, a charger will restore it in 27 minutes, or a mission will take 10 minutes, how often is that forecast interval correct on the actual deployed resource, at that site, in that weather/temperature/operating regime?

A low aggregate error on a random split is not sufficient. Battery prediction intervals can fail catastrophically when the test domain changes from random cycles to unseen cells, protocols, chemistries or EV-like profiles. The 2026 SOC/SOH conformal audit by Alhussein et al. reports 90.9% SOC coverage for a 90% nominal target on a random cycle split, but 33.0–81.4% on held-out cells, at most 0.9% on cross-protocol windows with nonzero SOC, at most 5.16% across chemistries and 0.02% on EV-profile cells. These are the authors' battery-dataset results, not mobility-fleet results. The important conclusion is that source-calibrated intervals do not transfer automatically under deployment shift. [Alhussein et al., Processes 14(21), 3557 (2026)](https://doi.org/10.3390/pr14213557).

## 2. Paradigm shift: forecast reliability is part of the capability proof

A scheduling or reserve gate should not consume a bare point estimate. It should consume a prediction with:
- exact asset/site/configuration identity;
- model and version identity;
- forecast issue time and information cutoff;
- target interval and measurement boundary;
- lower/point/upper prediction or probability;
- uncertainty/calibration provenance;
- later measured outcome with unique evidence reference;
- cohort/domain membership and shift warnings.

An unknown or unsupported deployment stratum must not inherit the confidence of a nominal laboratory or fleet-average group. The safe response is an explicit wide interval, conservative bound, reduced capability, or abstention—not a silent fallback to nominal.

This connects forecasting to the existing assurance chain without merging ownership:
- #6909 supplies the deterministic modelled opportunity timeline;
- #6999/#7004 bind request slots to those exact opportunities;
- #7074 consumes the event-derived opportunities;
- #7129 supplies a clairvoyant offline upper bound, not an online guarantee;
- #7133 constrains decisions to information revealed at each scenario-tree node;
- #6803/#6809/#6829/#6837 remain hard feasibility, recovery, unique commitment and journal gates.

This calibration module evaluates prediction/outcome evidence. It does not change a model, reserve a resource, or grant a physical operating decision.

## 3. SOTA comparison: separate accuracy, coverage, sharpness and shift response

### 3.1 FeatureFormer + conformalized quantile regression

Soon and Soon (Journal of Power Sources 666, article 239123, 28 February 2026) combine FeatureFormer with conformalized quantile regression for SOC uncertainty. The paper reports near-95% coverage, MCE 0.0194, Winkler score 13.636 and normalized mean prediction interval width 0.1452 on its own evaluation. These are model/dataset-specific results; they are not evidence that the same calibration transfers to another chemistry, pack generation, charger, climate or aircraft duty cycle. [DOI 10.1016/j.jpowsour.2025.239123](https://doi.org/10.1016/j.jpowsour.2025.239123).

Reproduce it as a strong in-domain baseline. Compare coverage and interval width on identical source-native data before attempting any cross-domain transfer. A wide interval that covers everything is not useful; a narrow interval that misses during deployment shift is unsafe. Report both.

### 3.2 Online calibration under temporal dependence and shift

Huang, Ma and Michailidis (UAI 2026, PMLR 337:2244–2273) propose an online martingale PAC-Bayes certificate for time-series forecasting under temporal dependence and distribution shift. Their predict-then-update protocol uses only outcomes observed after the forecast when updating subsequent predictions, and their certificate includes a source-risk anchor, a mismatch term and a complexity term. [PMLR paper](https://proceedings.mlr.press/v337/huang26b.html).

This is a key next comparator. The current v0.1 evaluator uses a frozen chronological calibration/holdout split and reports empirical coverage; it does not claim a martingale certificate, online adaptation or validity under arbitrary dependence. The v0.2 tournament should compare static split conformal, group-conditional conformal, ACI/strongly adaptive online conformal and certificate-driven predict-then-update methods on the same chronological trace.

### 3.3 Diagnose shift before it becomes a reliability failure

Lee (UAI 2026, PMLR 337:3453–3476) reports that identical temporal distribution shifts can produce very different conformal coverage loss across tasks, and that generic shift detectors may not distinguish catastrophic from benign failure. The paper explores SHAP feature-importance concentration as one model-specific diagnostic; this should be treated as an exploratory comparator, not a universal detector. [PMLR paper](https://proceedings.mlr.press/v337/lee26g.html).

A mobility deployment audit should measure both:
1. whether the input distribution shifted; and
2. whether uncertainty/coverage actually degraded on labeled target outcomes.

A shift alarm is not itself proof of prediction failure, and absence of a generic shift alarm is not proof of calibration.

### 3.4 Federated multi-site calibration

The UAI 2026 paper on valid and efficient uncertainty quantification under federated joint shift (PMLR 337:6232–6260) proposes privacy-preserving global sketches of nonconformity score distributions, followed by local importance-weighted calibration. It reports improved predictive efficiency on its own heterogeneous benchmarks. [PMLR paper](https://proceedings.mlr.press/v337/shi26a.html).

This creates a valuable later architecture: depots could share privacy-preserving calibration summaries without pooling raw aircraft, battery or customer telemetry. It must not be assumed that site-level score sketches solve arbitrary chemistry or concept shift; target-domain evidence and privacy review remain necessary.

## 4. Measurement boundaries and evidence model

The v0.1 input keeps distinct:

| Metric | Required boundary |
|---|---|
| Mission duration | Aircraft/mission clock |
| Pack energy consumed | Pack-terminal Wh |
| Charge restoration duration | Charger/service clock |
| Charging energy | Charger-input Wh |
| Queue wait | Charger/service clock |
| Peak site import | PCC-meter W |

Do not compare pack-terminal energy to charger-input energy. If the same physical interval is measured at two boundaries, retain both observations and the conversion/efficiency evidence; do not overwrite one with the other.

Each forecast and outcome envelope must bind to exact asset/resource, configuration digest, producer, method, unique reference and time validity. Forecast evidence is issued before the target interval. Outcomes are observed after the target interval and before the audit cutoff. Replayed evidence references are rejected.

The v0.1 fixture uses synthetic evidence. It qualifies schema, arithmetic, chronology and alarm behavior only. It cannot support any field reliability statement.

## 5. Chronological stratification

The benchmark must preserve a later holdout:
- model ID/version;
- site and site-configuration digest;
- asset and exact configuration digest;
- operating stratum;
- metric and measurement boundary;
- nominal coverage level.

Candidate strata for actual deployments include:
- cell chemistry, pack generation, BMS and firmware;
- battery age/SOH and charge/discharge protocol;
- temperature, wind, rainfall, altitude and payload;
- charger/inverter model and channel topology;
- site feeder/import envelope, grid interruption and renewable supply;
- mission class, duration and operational turnaround;
- sensor/model version and telemetry path.

Do not pool distinct strata to conceal undercoverage. A group with insufficient calibration or holdout samples is explicitly unsupported. Future South African deployment evidence should distinguish heat, high wind, load-shedding/PCC interruptions, local feeder class and local permission regime rather than importing a foreign fleet's calibration.

## 6. v0.1 executable scope

Files:
- \`tools/ev-passenger-cost-frontier/src/service_prediction_calibration.rs\`
- \`tools/ev-passenger-cost-frontier/src/bin/service_prediction_calibration_screen.rs\`
- \`tools/ev-passenger-cost-frontier/schemas/service_prediction_calibration_v0_1.schema.json\`
- \`tools/ev-passenger-cost-frontier/fixtures/service_prediction_calibration_v0_1.json\`

Run:

\`\`\`bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin service_prediction_calibration_screen -- \\
  tools/ev-passenger-cost-frontier/fixtures/service_prediction_calibration_v0_1.json
\`\`\`

For each model/version, site configuration, stratum, metric, physical boundary and resource configuration, the evaluator:
1. separates the chronological calibration cohort from the later holdout;
2. computes the finite-sample split-conformal nonconformity quantile from calibration records only;
3. expands the holdout prediction interval by that correction;
4. reports pre/post holdout coverage, Wilson 95% coverage interval, mean width, MAE and Winkler score;
5. computes holdout-only Brier score and reliability bins for on-time probabilities;
6. raises an undercoverage alarm when observed post-calibration coverage falls below nominal minus the declared tolerance;
7. abstains on insufficient sample support, invalid evidence, chronology leakage, duplicate receipts or measurement-boundary mismatch;
8. emits a canonical SHA-256 digest and never updates the model or grants field/operational authority.

The conformal calculation is an empirical split-calibration diagnostic. It does **not** claim distribution-free coverage under arbitrary temporal dependence, repeated measures, non-exchangeability or unknown domain shift.

## 7. Frozen synthetic deployment-shift fixture

The fixture contains:
- 100 sample records: 40 calibration, 60 later holdout;
- two strata: \`nominal\` and \`hot_shift\`;
- three metrics per record: mission duration, pack-terminal energy and PCC peak import;
- 20 calibration and 30 holdout samples per metric/stratum group;
- 90% nominal intervals and explicit forecast/outcome evidence envelopes.

The nominal holdout has all observations inside its prediction intervals after the frozen calibration calculation. In the hot-shift holdout, the simulated errors are deliberately much larger than the calibrated interval. The same on-time model assigns 90% probability to both strata, but the hot-shift on-time outcome rate is 60%.

Expected behavior:
- 6 metric groups and 2 on-time groups;
- nominal post-calibration interval coverage = 1000 permille for all three metrics;
- hot-shift post-calibration interval coverage = 0 permille for all three metrics and an undercoverage alarm;
- hot-shift observed on-time rate = 600 permille;
- synthetic evidence flag true, source-grade flag false;
- all field qualification, model mutation and operational authority flags false.

These are frozen synthetic known answers, not measured deployments or predictive performance. Exact-head Actions run [#45333671953](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45333671953) at `9f61b00ea3b4077f5f67409222e27dc20806f835` passed schema/fixture parsing, workspace formatting, Cargo tests, deterministic report assertions and artifact upload: 241 tests passed, 0 failed across test binaries. The module contains 17 focused Rust tests. Clippy remains unverified.

## 8. Negative controls

The module currently contains 17 focused Rust test definitions. They include:
- future forecast after target start;
- calibration outcome at or after earliest holdout forecast;
- exact metric measurement-boundary mismatch;
- duplicated observation evidence reference;
- wrong asset/configuration binding;
- insufficient calibration support;
- interval point outside its lower/upper bounds;
- deterministic report under input permutation;
- on-time probability scoring uses holdout outcomes only;
- interval coverage, Wilson lower bounds and interval width remain separate outputs;
- synthetic evidence cannot produce field qualification or empirical support.

The evaluator should be extended with more adversarial cases before production use: repeated measurements from one pack counted as independent samples, chemistry/configuration pooling, an unobserved regime, post-selection of a calibration cohort, changed model version, missing target labels, and a shift that preserves mean error but breaks tail coverage.

## 9. Benchmark tournament and target ladder

All methods must use identical forecast origins, target intervals, chronology, model versions, strata and evidence cut-offs.

| Comparator | Role |
|---|---|
| Uncalibrated point/quantile model | Base error and raw interval coverage |
| Static split conformal | Frozen in-domain calibration |
| Group-conditional split conformal | Resource/site/stratum-specific calibration |
| Conformalized quantile regression | Strong interval baseline |
| ACI / strongly adaptive online conformal | Sequential adaptation |
| Online certificate-driven calibration | Temporal dependence/shift-aware certificate comparator |
| Federated shift-aware calibration | Privacy-preserving cross-site follow-on |

Targets (not achievements):
- 100% chronology and evidence-reference integrity;
- zero cross-boundary metric comparisons;
- 90%/95% nominal interval targets reported with Wilson bounds and actual sample counts;
- worst-stratum undercoverage never hidden by pooled averages;
- at equal coverage, improve Winkler score/mean interval width against matched baselines;
- report Brier score, reliability bins and ECE for on-time probabilities;
- measure time-to-detect and time-to-recover after a shift;
- insufficient support must produce abstention or unsupported status;
- 100% agreement with an independent implementation of order-statistic/Wilson calculations for bounded fixtures.

A measured 95% interval target with a broad interval may be less useful than a 90% interval with a narrower width, depending on the risk class. Report coverage, sharpness and operational consequence separately; do not compress these into one score.

## 10. Next research increments

- **v0.2:** chronological online predict-then-update calibration with a strict event cutoff and immutable prior predictions.
- **v0.3:** independent martingale/PAC-Bayes certificate baseline and explicit time-dependence handling.
- **v0.4:** shift taxonomy and detection; separate covariate, concept, chemistry, sensor, climate and site changes.
- **v0.5:** privacy-preserving/federated calibration summaries across depots, with no raw telemetry exchange.
- **v0.6:** field pilot with calibrated pack-terminal/PCC instruments, serial-level exposure and independent holdout custody.

## 11. Claim boundary

A passing synthetic evaluator means only that the supplied chronological evidence and forecast intervals produce the reported empirical statistics on the frozen fixture. It does not establish model accuracy, guaranteed coverage under arbitrary shift, battery safety, flight reliability, grid qualification, South African permission, durable evidence authenticity or production readiness.