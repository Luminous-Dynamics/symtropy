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

Commercial services already cover important portions of vessel/engine health management:

| Provider/product | Publicly described capability | Comparison opportunity |
|---|---|---|
| Wärtsilä Expert Insight | AI anomaly detection and rule-based diagnostics against vessel-specific operating profiles, expert analysis and crew/shore collaboration | Compare alert burden, lead time, evidence traceability, cross-OEM portability and maintenance handoff |
| MAN CEON / PrimeServ Assist | Engine/turbine/compressor sensor monitoring, AI/ML analysis and global expert monitoring/advisory | Compare configuration identity, fault isolation, calibrated uncertainty, exportable evidence and independent replay |
| Kongsberg Vessel Insight / Health Management | Edge collection, compression/caching and ship-system connectors, with machinery-health/RUL analysis and experts-in-the-loop | Compare data lineage, offline/partition behavior, protocol interoperability and exact replay across edge/cloud |
| DNV Machinery Maintenance Connect / MPMS | Digital machinery planned-maintenance surveys using vessel maintenance data, fleet benchmarks and remote survey workflows | Compare auditability, change history, maintenance-evidence completeness and the boundary between advice and class/maintenance authority |

Sources: [Wärtsilä Expert Insight](https://www.wartsila.com/marine/services/lifecycle-agreements/expert-insight-service), [MAN PrimeServ Assist](https://www.man-es.com/services/strategic-expertise/digitalization/primeserv-assist/1), [Kongsberg Health Management brochure](https://www.kongsberg.com/contentassets/f07caba826f94c798d6ea4c93a7f16a1/201.global-customer-support-2p-16.12.21.pdf), [Kongsberg Vessel Insight data sheet](https://www.kongsberg.com/contentassets/1fcaa7b8c3ad44c1be76cfbd6d007673/vessel-insight-data-technical-sheet.pdf), [DNV Machinery Maintenance Connect](https://www.dnv.com/services/machinery-maintenance-connect-mmc--168870/).

These are vendor/provider descriptions, not independent diagnostic-accuracy results. No public, matched head-to-head dataset or universal performance score was identified for these proprietary services. We must not claim Symthaea/Symtropy already outperforms them. A credible differentiation is complementary: an **open, vendor-neutral evidence and assurance layer** that can consume OEM diagnostics as attributed inputs, independently check configuration/source/freshness/dependency claims, reconcile evidence to vessel capabilities and ODD constraints, and keep model recommendations separate from human, maintenance and class authority.

### Published model results: what is comparable?

| Work | Publicly reported result/protocol | Interpretation |
|---|---|---|
| Xie, Mo & Jia, CDAF, *Scientific Reports* (14 April 2026), [DOI 10.1038/s41598-026-46425-w](https://doi.org/10.1038/s41598-026-46425-w) | Reports 99.71% accuracy, 99.75% precision, 99.68% recall and 99.71% F1 on a hybrid dataset. The paper describes empirical vessel telemetry used to calibrate a 1-D thermodynamic model and generate a hybrid fault dataset; its main split is randomized 70/15/15 after preprocessing. Table 5 reports 95.85% accuracy under 15% Gaussian noise, while the conclusion says 94.85%. | Priority replication baseline, not evidence of unseen-run, unseen-load or unseen-vessel performance. Resolve the 1 percentage-point table/conclusion discrepancy. The authors identify class imbalance and rare weak operational faults as future work. |
| Xie, Mo & Jia, *Multi-source thermal and performance data for thermal fault diagnosis in marine diesel engines*, *Scientific Data* (22 August 2026), [DOI 10.1038/s41597-026-07924-7](https://doi.org/10.1038/s41597-026-07924-7) | Describes calibrated simulation, noisy bench measurements, real-vessel telemetry and virtual-real fault fusion across one healthy state and ten thermal fault modes; reports 70.18% zero-shot virtual-to-real accuracy under its own protocol. | High-value transfer baseline. It is not a score on the MU323DGSC archive. Audit the exact repository, data rights, label provenance and split before comparing. |
| Michel et al., *Proactive fault prediction in marine diesel engines using multivariate machine learning*, *Scientific Reports* (20 March 2026), [DOI 10.1038/s41598-026-40979-5](https://doi.org/10.1038/s41598-026-40979-5) | Reports 82.681% accuracy for a Random Forest model with same-study comparisons to 1-D CNN and MLP. | Useful additional literature baseline. Its data provenance, task labels and split must be checked before a direct comparison. |
| BahooToroody et al., *Marine Engine Fault Dataset*, Zenodo v1, [DOI 10.5281/zenodo.19857425](https://doi.org/10.5281/zenodo.19857425) | Five physically induced fault families on a controlled MU323DGSC engine test bench; 16 recordings, approximately 115,000 samples and about 70 channels. | Our first measured fault benchmark for file/run-held-out and load-transfer tests once the payload and channel dictionary are audited. No model score has yet been reproduced here. |
| 2026 multisource-fusion study, [PMC full text](https://pmc.ncbi.nlm.nih.gov/articles/PMC12811478/) | Reports 99.45% accuracy; its described experiment uses 2,567 samples generated from a 1-D thermodynamic simulation model with a 70/30 stratified split and cross-validation. | Simulation-based result, not a physical test-bench or shipboard score. Keep separate from empirical comparisons. |

The CDAF paper states that original code and **part** of the real-vessel data are public at [the authors' GitHub repository](https://github.com/xiezaimi-png/Marine-diesel-engine-explainable-weak-thermal-fault-diagnosis). The repository exists, but its files, exact commit, license for shared data, runnable environment and payload have not yet been audited in this environment. The associated *Scientific Data* descriptor is a distinct source and must be pinned to its own dataset release and license. Do not infer that an open-access article license automatically covers its underlying data.

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
- Improve the worst-performing fault family and unknown/OOD rejection, not only the average score.
- Show improvement on an untouched real-vessel domain before making any shipboard-transfer claim.
- Maintain zero evidence/authority-gate violations: no stale, wrong-configuration, wrong-unit, unauthenticated or simulator-only evidence promoted into current measured health; no false independence across common-cause sources; no automated maintenance authorization or ODD transition.
- No "beats SOTA" headline until both the payload and matched baselines are independently audited and reproduced.

The paradigm shift is **diagnostics that compile into evidence-qualified capability**: identify a fault hypothesis; expose source, calibration, configuration and uncertainty lineage; map affected services and dependency paths; project only a bounded capability impact to the ODD owner; require accountable inspection, proof-test, configuration and authorization evidence before a human-led recovery review. This is how Symtropy/Symthaea/Mycelix can complement OEM diagnostics without pretending to replace engine OEMs, chief engineers, class or flag authorities.

## Attribution and claim boundary

Cite the dataset DOI and the associated preprint. The dataset is CC BY 4.0; retain author attribution and license notices if redistributing derived material. This benchmark can establish performance on the released MU323DGSC controlled test-bench recordings only. It does not establish shipboard performance, fleet reliability, seaworthiness, safe operating limits, class/statutory compliance, MASS Code compliance, maintenance authorization or permission to operate.

See `benchmarks/maritime/marine-engine-fault-dataset-v1.manifest.json` for the machine-readable source record and current audit status.
