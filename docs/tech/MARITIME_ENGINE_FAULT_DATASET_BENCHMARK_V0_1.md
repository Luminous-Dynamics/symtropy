# External marine engine fault benchmark v0.1

**Dataset:** [Marine Engine Fault Dataset, Zenodo 19857425](https://doi.org/10.5281/zenodo.19857425)  
**Related preprint:** [arXiv:2607.19444](https://arxiv.org/abs/2607.19444)  
**License:** CC BY 4.0; attribution is required  
**Integration status:** metadata verified; archive payload and channel dictionary not yet audited

## Why this matters

The public record describes controlled experiments on a Matsui Iron Works MU323DGSC turbocharged/intercooled three-cylinder marine diesel engine. It contains one baseline reference recording and 15 fault-scenario CSVs, approximately 115,000 timestamped samples and about 70 measured/derived channels. Five fault families were physically induced: compressor air-filter clogging, air-cooler fouling, injection-valve nozzle clogging, cooling-water pump cavitation and turbine degradation. The Zenodo record describes 16 CSV files, 73 columns for scenario files (including three leading time/label columns), a distinct 70-column baseline schema, a three-row header, dataset index and variable dictionary.

This is substantially stronger than synthetic-only examples for testing machinery fault detection and transfer across load conditions. It is still one controlled test bench and one engine model—not a shipboard fleet validation set. The associated preprint is submitted to a journal and shown as under review; it is not a final peer-reviewed benchmark standard.

## Payload audit status

This manifest records the publisher's public metadata only. The 26.5 MB archive could not be retrieved by this execution environment because the web content fetch rejected the payload size and the local runtime could not resolve the external host. Therefore:
- raw CSVs have not been read;
- the dataset index and variable dictionary have not been inspected;
- the publisher's MD5 value has not been checked against downloaded bytes;
- no SHA-256 is claimed;
- no split assignments or performance results are claimed.

Do not treat the published file description as an actual local payload audit. Once retrieved in a connected environment, verify archive bytes, inspect archive paths, parse all three header rows, reconcile baseline/scenario schemas, check timestamps, missingness and exact per-file sample counts, then update the manifest with SHA-256 and audited metadata.

## Evaluation protocol

### 1. Freeze the data before modeling

Keep original CSV files as indivisible recording groups for primary evaluation. A random row/window split would leak adjacent time samples and recording-specific operating conditions across training and test sets. Preserve source run, fault intervention, load, engine configuration, timestamp range, label availability and channel-presence mask.

Do not merge the 70-column baseline file and 73-column scenario files by positional column index. Align channels only through the inspected variable dictionary, normalized name, unit and physical meaning. Retain source names and unit metadata. Any conversion must be explicit and reproducible.

### 2. Use multiple complementary evaluation regimes

- **Run-held-out fault diagnosis:** keep every window from an original scenario recording in one fold. Report fault family and load coverage; do not claim generalization beyond held-out runs from this test bench.
- **Load transfer:** for fault families with sufficient distinct load runs, train on some loads and hold out entire load-level recordings. A held-out load is a distinct test, not a random segment split.
- **Baseline nuisance-alert assessment:** because the record has one baseline recording, use chronological blocked windows with a purge/guard interval to quantify within-recording false alarms. Explicitly label this as a single-campaign estimate, not independent baseline replication.
- **Open-set severity/unknown analysis:** hold the two-hole injection-nozzle restriction out as a separate severity/generalization experiment. Do not score event-onset detection on the two injection-nozzle files documented as anomaly-only; they lack an observed pre-anomaly segment.
- **Missing-channel robustness:** evaluate the five runs with the two documented absent channels separately. Preserve masks and compare mask-aware inference with any imputation baseline; fit imputation only on the training partition.
- **Pre-anomaly versus anomaly detection:** use only files whose label timeline actually includes both states. Confirm the label transition and annotation semantics from each CSV before scoring latency.

Split assignments remain pending archive audit; do not hardcode inferred filenames or load labels until the index is inspected.

### 3. Matched model and workflow baselines

Run all methods over the same file groups, windows, channel mapping, label scope, train-only normalization and compute budget:

1. documented equipment alarms/engineer rules, only where source limits are explicitly available;
2. PCA / multivariate statistical process control;
3. Isolation Forest or One-Class SVM for anomaly detection;
4. Random Forest / gradient-boosted trees for labelled fault classification;
5. temporal convolutional / 1-D CNN baseline;
6. a calibrated open-set or abstaining classifier;
7. Symthaea evidence synthesis, on identical input channels and labels;
8. physics/model residuals only if an engine model and independent validation evidence exist.

Separate anomaly detection from fault isolation, severity classification, root-cause inference and remaining-useful-life estimation. The archive description alone does not establish labels suitable for every task.

### 4. Metrics

Report by fault family, load, fault realization, channel-availability pattern and evaluation regime:
- event-level recall and missed-fault rate;
- false alarms per engine-hour and false-alarm duration;
- detection latency from documented anomaly onset;
- fault-family macro precision/recall/F1 and balanced accuracy;
- calibration and uncertainty (Brier score, reliability curve, interval coverage where applicable);
- unknown/OOD rejection and false-known rate;
- confusion between physically different fault mechanisms;
- load-transfer performance delta;
- degradation under channel dropout and sensor-quality perturbations;
- inference latency and compute/memory footprint.

Use grouped bootstrap confidence intervals by recording, not by individual row. Report per-file results; the number of independent physical runs is small. Avoid an overall accuracy headline dominated by repeated, autocorrelated samples.

### 5. Proposed competitive criteria (targets, not claims)

- Zero cross-recording leakage in the primary held-out protocol.
- Every result binds dataset DOI/version, file digests, channel map, unit conversions, split manifest, model/configuration, code commit and evaluator digest.
- At matched fault recall, reduce false alarms per engine-hour versus the strongest reproducible baseline; alternatively, at matched false-alarm burden, improve event recall. Report the Pareto frontier, not one weighted score.
- Improve held-out-load performance without degrading the worst-performing fault family.
- Preserve an explicit unknown/abstain path; do not trade false-known confidence for nominal accuracy.
- No numerical "beats SOTA" target until the archive is independently audited and baseline results are reproduced on identical file-level splits.

## Industry, standards and SOTA comparison

This test bench enables matched experiments against conventional condition-monitoring and machine-learning approaches. It is not a universal marine-engine leaderboard. Keep four evidence categories distinct: **normative/process standards**, **commercial product capabilities**, **published paper results**, and **independently reproduced performance**.

### Standards baseline

| Standard / framework | Current edition or status | Relevance | What it does not establish |
|---|---|---|---|
| ISO 17359 | ISO 17359:2018, Edition 3; confirmed current in 2023 | General procedures for establishing a machine condition-monitoring programme | Diagnostic accuracy or a marine fault leaderboard |
| ISO 13374-1 | ISO 13374-1:2003; confirmed current in 2025 | Data processing, communication and presentation for condition-monitoring and diagnostic information | A fault classifier or machinery safety approval |
| ISO 13379-1 | ISO 13379-1:2025, Edition 2 | Concepts and guidance for diagnostic interpretation and method selection | A vessel-specific operating-envelope decision |
| ISO 13381-1 | ISO 13381-1:2025, Edition 3 | Prognostics process guidance | An independently validated remaining-useful-life result |
| DNV AROS | DNV class-notation family | Separates navigation, engineering, operational and safety functions; distinguishes remote control, decision support, supervised autonomy and full autonomy | A public matched machinery-diagnostics score |
| IACS UR E26/E27 | Rev.1 requirements for applicable newbuilds contracted from 1 July 2024 | Cyber resilience at ship and onboard-system/equipment level | Physical fault-diagnosis correctness |
| IMO MASS Code | MSC.595(111), non-mandatory, effective 1 July 2026 | Goal-based MASS design/operation context including engineering, remote operations, risk, cyber and human oversight | Compliance or approval from this benchmark |

These standards form a useful chain: **condition-monitoring programme → diagnostic data handling → diagnosis → prognosis**. The engineering-health evaluator is an evidence/assurance adapter in that chain, not a claim of standards conformance. DNV AROS and the IMO MASS Code reinforce that engineering capability must be considered alongside navigation, safety, operating modes and human responsibility. A health disposition can inform an ODD/mode owner; it cannot independently authorize a transition.

Official references: [ISO 17359](https://www.iso.org/standard/71194.html), [ISO 13374-1](https://www.iso.org/standard/21832.html), [ISO 13379-1](https://www.iso.org/standard/88027.html), [ISO 13381-1](https://www.iso.org/standard/88029.html), [DNV AROS](https://www.dnv.com/maritime/autonomous-remotely-operated-ships/aros-class-notation/), [IACS UR E26/E27](https://iacs.org.uk/news/iacs-ur-e26-and-e27-press-release), [IMO MASS Code](https://www.imo.org/en/mediacentre/pressbriefings/pages/imo-adopts-mass-code.aspx).

### Commercial baseline

The current competitive bar includes condition monitoring, AI anomaly detection, remote expert advisory, predictive maintenance, type approval and OEM-specific troubleshooting assistants—not just classification.

| Provider/product | Publicly described capability and current evidence | Comparison opportunity |
|---|---|---|
| Wärtsilä Expert Insight | Vessel-specific operating profiles, AI anomaly detection, rule-based diagnostics, expert investigation and crew/shore collaboration. These remain relevant examples of OEM-integrated condition monitoring. | Alert burden, lead time, sensor coverage, cross-OEM portability, evidence export and independent replay. |
| Everllence CEON / PrimeServ Assist | PrimeServ Assist is a 24/7 data/AI/expert service using CEON and hundreds of engine/turbine/compressor sensors, with condition dashboards, trend/scatter plots and expert response. Everllence also offers CEON TechBot, an AI assistant grounded in OEM manuals, service letters and technical instructions. On 17 March 2026 Bureau Veritas announced Smartship Software Type Approval and Smart Service Supplier recognition for PrimeServ Assist for BV SMART(MH3); the reported scope included CEON, monitoring/advisory on two- and four-stroke engines and two-stroke remote operations centres. BV stated software type approval covered about 80% of required verification/documentation, leaving vessel-specific elements. | Do not compete with OEM documentation QA alone. Benchmark independent evidence provenance, multi-OEM/third-party equipment coverage, operational vs reconstructed signals, calibrated OOD, evidence portability and vendor-neutral replay. Compare type-approved integration evidence separately from model performance. |
| Kongsberg K-CMS / Condition Monitoring Application | DNV type-approval certificate TAA00003FT (issued 5 June 2025, valid to 4 June 2027) covers K-CMS / CMA 2.31.x and security profile 1 aligned to IACS UR E27 Rev.1, subject to stated conditions. The certificate describes onboard condition decision support and predictive-maintenance inputs, with local display and optional cloud display via K-GSN. Vessel-specific architecture, asset inventory and hardening evidence are part of certification conditions. | Compare component/version binding, installation topology, sensor calibration and acquisition lineage, offline behavior, third-party data ingestion and reproducible evidence export. Do not conflate product type approval with fleet-wide diagnostic accuracy. |
| DNV Machinery Maintenance Connect / MPMS | Maintenance-history and planned-maintenance survey product. DNV's onboarding guidance requests at least one year of vessel machinery maintenance history in a defined template before an initial digital survey; surveys are performed by trained surveyors, with vessel and fleet dashboards. This is not a real-time fault-classifier leaderboard. | Compare maintenance-history provenance, change audit, survey readiness, fleet benchmarking and the boundary between advisory, survey and class authority. |

Sources: [Wärtsilä Expert Insight](https://www.wartsila.com/marine/services/lifecycle-agreements/expert-insight-service), [Everllence PrimeServ Assist](https://www.everllence.com/services/offerings/marine-power/digital-solutions/primeserv-assist/), [Everllence CEON TechBot and digital services](https://www.everllence.com/services/offerings/marine-power/digital-solutions/assist-plus), [Bureau Veritas / Everllence PrimeServ Assist type approval announcement, 17 March 2026](https://www.everllence.com/company/press-releases/details/2026/03/17/bureau-veritas-certifies-everllence-s-digital-monitoring-solution---primeserv-assist), [DNV K-CMS type approval certificate TAA00003FT](https://www.kongsberg.com/globalassets/kongsberg-maritime/documents/certificates/product/condition-monitoring-system/taa00003ft-web.pdf), [DNV Machinery Maintenance Connect](https://www.dnv.com/services/machinery-maintenance-connect-mmc--168870/), [DNV MMC onboarding](https://www.dnv.com/services/machinery-maintenance-connect-mmc--168870/existing-dnv-customers/).

These are vendor/provider descriptions and certification statements, not independent, matched diagnostic-accuracy results. The type approval applies to a defined product/version and its conditions; it does not imply a fleet-wide fault-diagnosis score. We must not claim Symthaea/Symtropy already outperforms these vendors.

The differentiation should be complementary and cross-OEM: a vendor-neutral evidence/assurance layer that accepts OEM diagnostics as attributed claims; validates asset/configuration, source, freshness and dependency lineage; distinguishes measured, derived and reconstructed values; exposes uncertainty and OOD; correlates evidence across propulsion, electrical, thermal, fuel, bilge and safety functions; and hands a bounded evidence contribution to vessel ODD and human maintenance authorities.

A credible superiority claim must be demonstrated on portable evidence, interoperability, replayability, low-connectivity operation, calibrated unknown detection, decision utility and lower false-alarm cost under matched conditions—not merely a better classification score on one public dataset.

### Published model results: what is comparable?

| Work | Publicly reported result/protocol | Interpretation |
|---|---|---|
| Xie, Mo & Jia, CDAF, *Scientific Reports* (14 April 2026), [DOI 10.1038/s41598-026-46425-w](https://doi.org/10.1038/s41598-026-46425-w) | Reports 99.71% accuracy, 99.75% precision, 99.68% recall and 99.71% F1 on a hybrid dataset. The paper describes empirical vessel telemetry used to calibrate a 1-D thermodynamic model and generate a hybrid fault dataset; its main split is randomized 70/15/15 after preprocessing. Table 5 reports 95.85% accuracy under 15% Gaussian noise, while the conclusion says 94.85%. | Priority replication baseline, not evidence of unseen-run, unseen-load or unseen-vessel performance. Resolve the 1 percentage-point table/conclusion discrepancy. The authors identify class imbalance and rare weak operational faults as future work. |
| Xie, Mo & Jia, *Multi-source thermal and performance data for thermal fault diagnosis in marine diesel engines*, *Scientific Data* (22 August 2026), [DOI 10.1038/s41597-026-07924-7](https://doi.org/10.1038/s41597-026-07924-7) | Describes calibrated simulation, noisy bench measurements, real-vessel telemetry and virtual-real fault fusion across one healthy state and ten thermal fault modes; reports 70.18% zero-shot virtual-to-real accuracy under its own protocol. | High-value transfer baseline. It is not a score on the MU323DGSC archive. Audit the exact repository, data rights, label provenance and split before comparing. |
| Michel et al., *Proactive fault prediction in marine diesel engines using multivariate machine learning*, *Scientific Reports* (20 March 2026), [DOI 10.1038/s41598-026-40979-5](https://doi.org/10.1038/s41598-026-40979-5) | Reports 82.681% accuracy for a Random Forest model with same-study comparisons to 1-D CNN and MLP. | Useful additional literature baseline. Its data provenance, task labels and split must be checked before a direct comparison. |
| BahooToroody et al., *Marine Engine Fault Dataset*, Zenodo v1, [DOI 10.5281/zenodo.19857425](https://doi.org/10.5281/zenodo.19857425) | Five physically induced fault families on a controlled MU323DGSC engine test bench; 16 recordings, approximately 115,000 samples and about 70 channels. | Our first measured fault benchmark for file/run-held-out and load-transfer tests once the payload and channel dictionary are audited. No model score has yet been reproduced here. |
| 2026 multisource-fusion study, [PMC full text](https://pmc.ncbi.nlm.nih.gov/articles/PMC12811478/) | Reports 99.45% accuracy; its described experiment uses 2,567 samples generated from a 1-D thermodynamic simulation model with a 70/30 stratified split and cross-validation. | Simulation-based result, not a physical test-bench or shipboard score. Keep separate from empirical comparisons. |

The CDAF paper states that original code and **part** of the real-vessel data are public at [the authors' GitHub repository](https://github.com/xiezaimi-png/Marine-diesel-engine-explainable-weak-thermal-fault-diagnosis). The repository exists, but its files, exact commit, license for shared data, runnable environment and payload have not yet been audited in this environment. The associated *Scientific Data* descriptor is a distinct source and must be pinned to its own dataset release and license. Do not infer that an open-access article license automatically covers its underlying data.

### Archive preview inventory — metadata only

The Zenodo archive preview is now inspectable even though the 26.5 MB ZIP payload could not be retrieved in this environment. Its preview tree lists **22 members**: 15 scenario CSVs, one baseline CSV, README, CITATION, LICENSE, dataset index, variable dictionary and the normalization script. The individual paths and preview-reported sizes are frozen in the dataset manifest.

This improves the inventory boundary but does **not** upgrade the payload-audit status:
- Preview tree confirmed: yes.
- Archive bytes downloaded: no.
- Publisher MD5 `f4d246c1bb46e05b26e56221acc2606c` independently checked: no.
- SHA-256, CSV row counts, per-file timestamp monotonicity, three-row header parse, label-transition positions, channel units/presence and normalization-script behavior: not audited.
- Split assignments and benchmark scores: not produced.

The row-level target is a **binary anomaly-state label**, while fault-family identity is represented by the scenario/recording metadata. Do not score row-level multi-class root-cause classification from this binary field, and do not treat it as an independently adjudicated severity label without auditing the README and annotation protocol. The two injector recordings contain anomalous operation only, so they cannot support pre-anomaly-to-anomaly onset scoring.

The archive record itself identifies the controlled MU323DGSC platform, the 16 CSV recordings, the 73-column scenario schema, the 70-column reference schema, the two anomaly-only injector files and five runs with two missing channels. Those are publisher metadata, not independently re-parsed measurements. [Zenodo record](https://zenodo.org/records/19857425) · [Zenodo archive preview](https://zenodo.org/records/19857425/preview/Marine_Engine_Fault_Data_v1.zip?include_deleted=0).

### Current transfer, prognostics and field-landscape references

Three additional studies broaden the benchmark requirements beyond static classification:

- **Cross-working-condition transfer:** the 2025 Applied Energy paper *A universal transfer learning framework for cross-working-condition marine diesel engine fault diagnosis based on fine-tuning strategy* reports 18 transfer tasks across an 8V396SE 500 kW engine and a 620 V16 2336 kW engine, with a mixture of experimental and virtual evidence depending on subsystem. The task coverage is useful, but we must reproduce each task separately and not pool the experimental and virtual paths. [DOI](https://doi.org/10.1016/j.apenergy.2025.125962).
- **Two-stroke multi-fault prognostics:** the June 2026 *Journal of Ocean Engineering and Science* study combines ensemble anomaly detection, clustering, load-aware classification and RUL forecasting. It highlights false positives below 1%; its accessible abstract reports representative RUL RMSE 0.215 and R² 0.413. These are paper-scale testbed results; the RMSE scale, RUL horizon, run-to-failure ground truth and false-positive exposure denominator must be audited before reproduction. [DOI](https://doi.org/10.1016/j.joes.2025.12.002).
- **Field landscape:** the October 22, 2026 review *Literature Review Analysis: Fault Diagnostics of Marine Engines* screened 2,114 initial records and reports a refined core corpus of 253 papers. It finds a persistent gap between algorithm development and shipboard validation, and calls for multi-engine/multi-vessel data, reusable benchmark datasets, physically informed and interpretable models, and maintenance-linked outputs. [DOI](https://doi.org/10.3390/jmse14211988).

The two-stroke paper makes an important point: we should not use one scalar "accuracy" score to claim superiority. Detection false positives, diagnosis quality and RUL prediction are distinct tasks with different labels, denominators and uncertainty.

### Causal risk from incident and maintenance narratives

A further 2026 direction is [A Novel Time-Varying Failure Risk Assessment Framework for Marine Diesel Engines Integrating Large Language Models and Bayesian Networks](https://doi.org/10.3390/jmse14171586), published 27 August 2026. Its hard-starting case study reports 338 cause descriptions from 35 records, 12 risk-influencing factors after extraction/expert consolidation, semantic coverage above 93% across four models, and service-time-dependent Bayesian risk estimates. The paper reports conditional posteriors of 60.92% for fuel-injection mechanical failure and 36.91% for governor/actuation failure after a hard-starting symptom, and a model-inferred single-start hard-start probability rising from 38.09% to 75.23% over 0–10,000 service hours under its parameterization.

These are **conditional model inferences**, not observed fleet failure rates. The benchmark must separately assess:
- source-span attribution and hallucination/omission;
- extraction precision/recall and expert agreement;
- causal-graph edge direction/support versus mere correlation;
- time-varying Bayesian calibration, prior sensitivity and Brier/log score;
- copied-report duplicate suppression and common-source dependence;
- whether a symptom-conditioned posterior is clearly distinguished from unconditional incidence.

LLMs may extract and organize candidate facts but cannot create incident evidence, assert physical causality without support, or authorize maintenance. This research is tracked separately in [MARITIME-CAUSAL-RISK-001 #4548](https://github.com/Luminous-Dynamics/symtropy/issues/4548).

### A harder SOTA protocol

Use two gates. A same-paper random split is a reproduction target; independent recording/domain generalization is a separate achievement.

**Gate A — reproduce the published task.**
1. Obtain the exact CDAF repository commit, shared data, channel dictionary, class definitions, preprocessing and split configuration.
2. Verify code and data licenses separately. Pin the exact source revision, file inventory and SHA-256 digests.
3. Reproduce the published 70/15/15 split, preprocessing and reported metrics, including the noise experiment. Record the 95.85% versus 94.85% reporting discrepancy rather than silently choosing one.
4. Re-run the named paper baselines (CNN-LSTM, WCNN-LSTM, CNN-GRU-KAN and TSRF) with identical features, labels, compute budget and seeds.

**Gate B — test generalization that the published split does not establish.**
1. Hold out complete physical recordings/runs; never split adjacent windows from one run across training and test.
2. Hold out whole load levels where a fault family has sufficient independent runs.
3. For the hybrid source, separately test simulation→bench, simulation→real-vessel and mixed-source→real-vessel. The target domain stays unseen during normalization, feature selection, calibration and threshold choice.
4. Hold out a second vessel/engine configuration when available. One vessel and one test-bench engine cannot establish fleet transfer.
5. Test unseen fault severity, rare/weak faults, missing channels, sensor bias, calibration changes and noise regimes.
6. Fit scaling, imputation, feature selection, calibration and thresholding on training/validation only. Preserve recording, engine, load, timestamp, source-domain and missing-channel identity in the split manifest.

Report more than accuracy: macro-F1 and balanced accuracy by fault; event recall and missed-fault rate; false alarms per engine-hour and alarm duration; detection latency/lead time; domain/load transfer delta; calibration (Brier score/reliability/coverage); unknown/OOD false-known rate and abstention coverage; missing-channel robustness; worst-fault performance; inference latency, memory and edge-compute cost. Use grouped bootstrap confidence intervals by recording/engine, not row-level confidence intervals.

### Competitive targets (targets, not claims)

- Reproduce the published 99.71% CDAF result on its exact split as a replication check, not a competitive win.
- Reproduce the reported 70.18% zero-shot virtual-to-real result on the exact dataset and protocol before claiming transfer improvement.
- Then beat the strongest reproducible baseline on **held-out runs and held-out loads**, with fewer false alarms at matched recall. Publish the full recall/false-alarm Pareto frontier and grouped confidence intervals.
- Proposed internal superiority target, only after sufficient independent run groups exist: at least **20% fewer false alarms per engine-hour at matched event recall**, with no material regression in worst-fault recall. This is an aspirational target, not an observed result or a universal operating threshold.
- Improve the worst-performing fault family and unknown/OOD rejection, not only the average score.
- Show improvement on an untouched real-vessel domain before making any shipboard-transfer claim.
- Maintain zero evidence/authority-gate violations on the frozen adversarial suite: no stale, wrong-configuration, wrong-unit, unauthenticated or simulator-only evidence promoted into current measured health; no false independence across common-cause sources; no automated maintenance authorization or ODD transition.
- No "beats SOTA" headline until both the payload and matched baselines are independently audited and reproduced.

### 2026 transfer and continual-learning challenge set

Two additional 2026 directions materially change the benchmark plan:

1. **Cross-individual digital-twin transfer.** Sun et al., *Research on cross-individual fault diagnosis of marine main engines based on adaptive mechanisms*, reports 100% DT-DANN diagnostic accuracy and 0.052% MAPE for adaptive tuning of individual-engine performance factors. The article explicitly describes the diagnostic result as simulation-based. These numbers must remain separate: twin-parameter MAPE is not classification accuracy, and the 100% result is not physical unseen-engine validation. The full engine-level split, twin-calibration leakage controls and independent physical validation remain to be audited. [DOI](https://doi.org/10.19693/j.issn.1673-3185.04917).
2. **Continual condition adaptation.** Shi et al., *Synaptic regularization for continual transfer*, proposes path-integral importance estimation and dynamic parameter masking for incremental marine-engine operating conditions under fixed model capacity and no historical-data replay. It compares against fine-tuning and regularization-based continual-learning methods and evaluates average accuracy and forgetting. This creates a separate benchmark axis: adaptation to new load/condition domains without erasing previously qualified conditions. Do not translate its qualitative abstract claims into numerical scores; reproduce the paper's exact protocol first. [DOI](https://doi.org/10.1016/j.energy.2026.141124).

3. **Multi-source adaptive transfer on measured data.** The 2026 *Energy Conversion and Management* study on four-stroke marine-diesel lubrication diagnosis reports 83.64% average accuracy for pure real measured scenarios and 82.37% for real/simulation hybrid scenarios on an instrumented 8V396 testbed at 25%, 50% and 75% loads. It proposes a shared-specialized feature network with entropy-weighted dynamic source-domain contributions and semi-supervised target adaptation. Its domain/task differs from Aalto MU323DGSC, so these are literature replication targets, not directly comparable scores. Reproduce source-only, equal-weight multi-source and entropy-weighted multi-source variants under the paper's exact protocol; then test negative transfer on held-out conditions. [DOI](https://doi.org/10.1016/j.enconman.2026.121194).
4. **Vessel-data quality and missing-value recovery.** The 2026 *Ocean Engineering* paper on localized anomaly detection and recovery uses GMM-EM to identify seven operating regions and SVD-based methods for localized anomaly detection and missing-value recovery on an oceangoing-vessel dataset. The accessible abstract does not give a numerical performance score. This is a data-quality comparator, not a directly comparable fault classifier. It motivates explicit testing of whether cleaning/imputation hides sensor failures or faults. [DOI](https://doi.org/10.1016/j.oceaneng.2026.124356).

Add four distinct experiments rather than another blended leaderboard:

- **Multi-source transfer / negative-transfer track:** compare source-only, best-single-source, equal-weight fusion, entropy-weighted source adaptation and target-supervised/few-shot adaptation. Hold target fault labels back in zero-shot tests, and report source contribution weights, target label budget, per-source domain gap and worst-target-condition performance.
- **Data-quality qualification track:** evaluate sensor dropout, intermittent missingness, stuck-at values, calibration drift, timestamp gaps and operating-mode transitions. Compare no imputation, mask-aware models and explicit GMM/SVD-style recovery. Report reconstruction error separately from fault-detection metrics, and measure whether imputation increases false-safe decisions or suppresses anomaly recall.
- **Cross-engine transfer track:** calibrate an individual digital twin using only permitted target-engine healthy-state/reference data; freeze target fault labels and evaluation recordings; hold out whole engines where possible; compare zero-shot, few-shot, source-only and domain-adapted methods. Report target calibration error separately from fault-classification metrics. A model that uses target fault labels during twin calibration is not zero-shot.
- **Continual-learning track:** order tasks by load, speed or operating regime; compare naive fine-tuning, SI/EWC/MAS-style regularization and the reported synaptic-tagging/capture method. After every update, re-evaluate all prior domains, fault classes, calibration and false-alarm rates. Report average accuracy, backward transfer, forgetting, worst-domain recall, update compute/storage and whether raw historical data were retained or replayed.

**Data integrity invariant:** a repaired or imputed value is never silently reclassified as a measured observation. Preserve the original missingness mask, transformation identity/version, input window, method parameters, training-data scope, uncertainty estimate and source digest. A synthetic/reconstructed value may support a model, but it cannot independently corroborate the underlying physical state or close an evidence gap in the engineering-health evaluator.

Proposed internal continual-learning target (not an existing result): after each new operating-condition update, keep prior-domain macro-F1 degradation within **2 percentage points**, and reduce average forgetting by at least **20%** against the strongest reproduced fixed-capacity/no-replay baseline. These targets apply only to the frozen research protocol; they do not qualify an operational system.

- Reproduce the published 99.71% CDAF result on its exact split as a replication check, not a competitive win.
- Reproduce the reported 70.18% zero-shot virtual-to-real result on the exact dataset and protocol before claiming transfer improvement.
- Then beat the strongest reproducible baseline on **held-out runs and held-out loads**, with fewer false alarms at matched recall. Publish the full recall/false-alarm Pareto frontier and grouped confidence intervals.
- Improve the worst-performing fault family and unknown/OOD rejection, not only the average score.
- Show improvement on an untouched real-vessel domain before making any shipboard-transfer claim.
- Maintain zero evidence/authority-gate violations: no stale, wrong-configuration, wrong-unit, unauthenticated or simulator-only evidence promoted into current measured health; no false independence across common-cause sources; no automated maintenance authorization or ODD transition.
- No "beats SOTA" headline until both the payload and matched baselines are independently audited and reproduced.

The paradigm shift is **diagnostics that compile into evidence-qualified capability**: identify a fault hypothesis; expose source, calibration, configuration and uncertainty lineage; map affected services and dependency paths; project only a bounded capability impact to the ODD owner; require accountable inspection, proof-test, configuration and authorization evidence before a human-led recovery review. This is how Symtropy/Symthaea/Mycelix can complement OEM diagnostics without pretending to replace engine OEMs, chief engineers, class or flag authorities.

## Reproducible dataset payload audit

The Rust-first audit executable is added at
`benchmarks/maritime-engineering-health/src/bin/marine_engine_dataset_audit.rs`.
After retrieving and safely extracting the licensed archive, run:

~~~sh
cargo run --locked \
  --manifest-path benchmarks/maritime-engineering-health/Cargo.toml \
  --bin marine_engine_dataset_audit -- \
  /path/to/extracted-dataset-root \
  benchmarks/maritime/marine-engine-fault-dataset-v1.manifest.json \
  --archive /path/to/Marine_Engine_Fault_Data_v1.zip \
  --output /path/to/marine-engine-dataset-audit-v0.1.json
~~~

The tool hashes each declared member and supplied archive, requires a valid ZIP signature plus exact publisher-MD5 match before returning success, checks the 3-row/73-column and 3-row/70-column layouts, validates binary anomaly-state labels, verifies onset-evaluable label coverage for the 13 recordings expected to contain both states, records the two anomaly-only nozzle files without inventing onset, audits absolute timestamp empties/duplicates and numeric relative-time ordering, counts common missing-value tokens by channel, and emits a split template with fault recordings unassigned and the baseline reference-only. It rejects path traversal and symlink escape, bounds file reads to 128 MiB, and does not download or extract archive content itself.

The report schema is
`docs/tech/MARITIME_ENGINE_DATASET_AUDIT_REPORT_V0_1.schema.json`. The report warns about duplicate timestamps, incomplete header metadata and extraction provenance that must be preserved separately.
A successful structural audit is still pending independent review; the dataset index and variable dictionary require semantic reconciliation. The report never auto-assigns train/test splits. Prognostics/RUL contracts and decision metrics are documented separately in `MARITIME_PROGNOSTICS_BENCHMARK_V0_1.md` and `MARITIME_RUL_FORECAST_V0_1.schema.json`.

Zenodo's file card lists the archive at 26.5 MB, while its separate record-statistics block displays 37.7 GB as data volume. The manifest records both separately and does not interpret the aggregate statistics value as archive payload size. The file-level preview inventory is not a substitute for retrieving and hashing the actual archive bytes.

## Machine-readable baseline registry

The literature and public-dataset records used above are frozen in [`marine-engine-diagnostic-baselines-v0.1.json`](../../benchmarks/maritime/marine-engine-diagnostic-baselines-v0.1.json) and validated against [`MARITIME_ENGINE_DIAGNOSTIC_BASELINES_V0_1.schema.json`](./MARITIME_ENGINE_DIAGNOSTIC_BASELINES_V0_1.schema.json). Each entry separates reported metric values from the study's task/data domain, split protocol, known limitations and current reproducibility status. The registry is not a leaderboard and does not imply that the reported values are independently reproduced.

## Attribution and claim boundary

Cite the dataset DOI and the associated preprint. The dataset is CC BY 4.0; retain author attribution and license notices if redistributing derived material. This benchmark can establish performance on the released MU323DGSC controlled test-bench recordings only. It does not establish shipboard performance, fleet reliability, seaworthiness, safe operating limits, class/statutory compliance, MASS Code compliance, maintenance authorization or permission to operate.

See `benchmarks/maritime/marine-engine-fault-dataset-v1.manifest.json` for the machine-readable source record and current audit status.
